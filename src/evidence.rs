//! Evidence promotion procedures (ADR-0004 §9): a primitive climbs the
//! ladder only through a recorded procedure, and failed procedures are
//! recorded too — negative results are valuable outputs.
//!
//! - `reproduce`        → Level 2 Replicated: re-run the exact protocol
//!   from the primitive's experiment manifest; the structure must
//!   re-emerge (same class, signature similarity ≥ 0.92). A FAILED
//!   reproduction demotes to Level 1 — replication failures matter.
//! - `perturbation`     → Level 3 Perturbation-Stable: re-run the
//!   protocol across shifted seeds × noise amplitudes (§8 trial
//!   ensemble); survival ≥ 60% promotes and the ensemble survival rate
//!   REPLACES the registration-time single-rerun noise_tolerance.
//! - `cross_resolution` → Level 4 Resolution-Stable: re-run at 0.75× and
//!   1.5× grid size; the structure must re-emerge at both (signature
//!   similarity ≥ 0.85, class-agnostic — classifier thresholds are
//!   resolution-sensitive, geometry is what must survive).
//!
//! Levels 5+ (cross-solver, behavioral, calibrated, hardware) need
//! machinery that does not exist yet; these procedures do not pretend to
//! reach them.

use crate::discovery::{evolve, replay_genome, EvolutionConfig, Genome};
use crate::engine::CrystalEngine;
use crate::lang::run_program;
use crate::manifest::ExperimentManifest;
use crate::primitives::signature_similarity;
use crate::registry::{data_dir, EvidenceRecord, Primitive, Registry};
use chrono::Utc;
use std::collections::HashMap;

// v2: evolve-manifest procedures replay the primitive's recorded GENOME —
// a closed protocol — instead of the full evolutionary search, whose
// fitness couples to live registry state (novelty term) and therefore
// diverges once the run's own discoveries have landed. Manifests without
// genome records (pre-4.1) fall back to full-evolution replay; the
// record's `method` field says which ran.
pub const REPRODUCE_PROCEDURE: &str = "reproduce-v2";
pub const PERTURBATION_PROCEDURE: &str = "perturbation-ensemble-v2";
pub const RESOLUTION_PROCEDURE: &str = "cross-resolution-v2";

const REPRODUCE_SIMILARITY: f64 = 0.92;
const CROSS_RES_SIMILARITY: f64 = 0.85;
const SURVIVAL_THRESHOLD: f64 = 0.6;
pub const PERTURBATION_NOISE_LEVELS: [f64; 4] = [0.005, 0.01, 0.02, 0.05];

enum Protocol {
    Evolve {
        cfg: EvolutionConfig,
        /// Genome records from the manifest's results (Phase 4.1), keyed
        /// by genome id — empty on pre-4.1 manifests.
        genomes: HashMap<String, Genome>,
    },
    Program {
        source: String,
        material_id: String,
        field_size: usize,
    },
}

fn load_protocol(prim: &Primitive) -> Result<Protocol, String> {
    let experiment_id = prim.experiment_id.ok_or_else(|| {
        format!(
            "{} has no experiment manifest (registered before ADR-0004 Phase 1) — \
             it cannot be promoted; regenerate it through a manifested run",
            prim.id
        )
    })?;
    let path = data_dir()
        .join("experiments")
        .join(format!("{experiment_id}.json"));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let manifest: ExperimentManifest =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;

    match manifest.program["kind"].as_str() {
        Some("evolve") => {
            let cfg: EvolutionConfig = serde_json::from_value(manifest.program["config"].clone())
                .map_err(|e| format!("manifest evolve config: {e}"))?;
            let genomes: HashMap<String, Genome> =
                serde_json::from_value(manifest.results["genomes"].clone()).unwrap_or_default();
            Ok(Protocol::Evolve { cfg, genomes })
        }
        Some("crystal-program") => Ok(Protocol::Program {
            source: manifest.program["source"]
                .as_str()
                .ok_or("manifest program source missing")?
                .to_string(),
            material_id: manifest.material.id.clone(),
            field_size: manifest.field_size,
        }),
        other => Err(format!(
            "experiment kind {other:?} is not reproducible to a primitive"
        )),
    }
}

/// Re-run a protocol, optionally perturbed. Returns the best signature
/// similarity to `prim` among structures the re-run produced, whether
/// the best match shares the class, and which method ran
/// ("genome-replay" — closed — or "full-rerun" — pre-4.1 fallback).
fn rerun(
    protocol: &Protocol,
    prim: &Primitive,
    seed_shift: u64,
    noise: f64,
    size_scale: f64,
) -> Result<(f64, bool, &'static str), String> {
    let compare = |structures: &[(Vec<f64>, crate::primitives::PrimitiveClass)]| {
        structures
            .iter()
            .map(|(sig, class)| {
                (
                    signature_similarity(sig, &prim.signature),
                    *class == prim.class,
                )
            })
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .unwrap_or((0.0, false))
    };
    match protocol {
        Protocol::Evolve { cfg, genomes } => {
            let mut cfg = cfg.clone();
            cfg.seed = cfg.seed.wrapping_add(seed_shift);
            cfg.ambient_noise = noise;
            cfg.field_size = ((cfg.field_size as f64 * size_scale) as usize).max(32);
            let recorded = prim.genome_id.and_then(|gid| genomes.get(&gid.to_string()));
            if let Some(genome) = recorded {
                // Closed protocol: replay the generating simulation.
                let (_, structures, _) = replay_genome(genome, &cfg);
                let pairs: Vec<_> = structures
                    .iter()
                    .map(|st| (st.signature.clone(), st.class))
                    .collect();
                let (sim, class_match) = compare(&pairs);
                Ok((sim, class_match, "genome-replay"))
            } else {
                // Pre-4.1 manifest: full-search replay (registry-coupled;
                // expected to diverge if the registry has since grown).
                let mut throwaway = Registry::default();
                evolve(&cfg, &mut throwaway, |_| {});
                let pairs: Vec<_> = throwaway
                    .primitives
                    .iter()
                    .map(|p| (p.signature.clone(), p.class))
                    .collect();
                let (sim, class_match) = compare(&pairs);
                Ok((sim, class_match, "full-rerun"))
            }
        }
        Protocol::Program {
            source,
            material_id,
            field_size,
        } => {
            let size = ((*field_size as f64 * size_scale) as usize).max(32);
            let mut engine = CrystalEngine::new(material_id, size, seed_shift)?;
            engine.noise_amp = noise;
            let mut throwaway = Registry::default();
            run_program(source, &mut engine, &mut throwaway).map_err(|e| e.to_string())?;
            let pairs: Vec<_> = throwaway
                .primitives
                .iter()
                .map(|p| (p.signature.clone(), p.class))
                .collect();
            let (sim, class_match) = compare(&pairs);
            Ok((sim, class_match, "full-rerun"))
        }
    }
}

fn node_name() -> String {
    std::env::var("KANNAKA_CRYSTAL_NODE").unwrap_or_else(|_| "local".into())
}

fn apply(
    registry: &mut Registry,
    id: &str,
    record: EvidenceRecord,
    achieved: Option<u8>,
    demote_to: Option<u8>,
    ensemble_noise_tolerance: Option<f64>,
) -> Result<EvidenceRecord, String> {
    let prim = registry
        .find_mut(id)
        .ok_or_else(|| format!("unknown primitive: {id}"))?;
    if let Some(level) = achieved {
        prim.evidence_level = prim.evidence_level.max(level);
    }
    if let Some(level) = demote_to {
        prim.evidence_level = prim.evidence_level.min(level);
    }
    if let Some(tol) = ensemble_noise_tolerance {
        prim.noise_tolerance = tol;
    }
    prim.evidence_records.push(record.clone());
    Ok(record)
}

/// Level 2: exact-protocol replication. Failure demotes to Level 1.
pub fn reproduce(registry: &mut Registry, id: &str) -> Result<EvidenceRecord, String> {
    let prim = registry
        .find(id)
        .ok_or_else(|| format!("unknown primitive: {id}"))?
        .clone();
    let protocol = load_protocol(&prim)?;
    let (similarity, class_match, method) = rerun(&protocol, &prim, 0, 0.0, 1.0)?;
    let success = similarity >= REPRODUCE_SIMILARITY && class_match;
    let record = EvidenceRecord {
        level: 2,
        procedure: REPRODUCE_PROCEDURE.into(),
        metrics: serde_json::json!({
            "success": success,
            "similarity": similarity,
            "class_match": class_match,
            "threshold": REPRODUCE_SIMILARITY,
            "method": method,
        }),
        at: Utc::now(),
        node: node_name(),
    };
    if success {
        apply(registry, id, record, Some(2), None, None)
    } else {
        // ADR-0004: demotion is possible when a record fails replication.
        apply(registry, id, record, None, Some(1), None)
    }
}

/// Level 3: §8 trial ensemble — shifted seeds × noise amplitudes.
pub fn perturbation(
    registry: &mut Registry,
    id: &str,
    seeds: u64,
    mut progress: impl FnMut(String),
) -> Result<EvidenceRecord, String> {
    let prim = registry
        .find(id)
        .ok_or_else(|| format!("unknown primitive: {id}"))?
        .clone();
    let protocol = load_protocol(&prim)?;
    let mut similarities = Vec::new();
    let mut survivals = 0usize;
    let total = seeds as usize * PERTURBATION_NOISE_LEVELS.len();
    let mut method = "genome-replay";
    for s in 0..seeds {
        for noise in PERTURBATION_NOISE_LEVELS {
            let (similarity, class_match, m) = rerun(&protocol, &prim, 1000 + s, noise, 1.0)?;
            method = m;
            let survived = similarity >= REPRODUCE_SIMILARITY && class_match;
            if survived {
                survivals += 1;
            }
            similarities.push(similarity);
            progress(format!(
                "  seed+{s} noise {noise}: similarity {similarity:.3}{}",
                if survived { "" } else { " (lost)" }
            ));
        }
    }
    let survival = survivals as f64 / total as f64;
    let mean = similarities.iter().sum::<f64>() / total as f64;
    let worst = similarities.iter().cloned().fold(f64::INFINITY, f64::min);
    let std = (similarities
        .iter()
        .map(|s| (s - mean) * (s - mean))
        .sum::<f64>()
        / total as f64)
        .sqrt();
    let success = survival >= SURVIVAL_THRESHOLD;
    let record = EvidenceRecord {
        level: 3,
        procedure: PERTURBATION_PROCEDURE.into(),
        metrics: serde_json::json!({
            "success": success,
            "survival_rate": survival,
            "mean_similarity": mean,
            "std_similarity": std,
            "worst_similarity": worst,
            "failure_rate": 1.0 - survival,
            "seeds": seeds,
            "noise_levels": PERTURBATION_NOISE_LEVELS,
            "threshold": SURVIVAL_THRESHOLD,
            "method": method,
        }),
        at: Utc::now(),
        node: node_name(),
    };
    // The ensemble survival rate is the honest noise tolerance (§8),
    // replacing the registration-time single-rerun number either way.
    apply(
        registry,
        id,
        record,
        if success { Some(3) } else { None },
        None,
        Some(survival),
    )
}

/// Level 4: cross-resolution stability at 0.75× and 1.5× grid size.
pub fn cross_resolution(registry: &mut Registry, id: &str) -> Result<EvidenceRecord, String> {
    let prim = registry
        .find(id)
        .ok_or_else(|| format!("unknown primitive: {id}"))?
        .clone();
    let protocol = load_protocol(&prim)?;
    let mut sims = Vec::new();
    let mut method = "genome-replay";
    for scale in [0.75, 1.5] {
        let (similarity, _, m) = rerun(&protocol, &prim, 0, 0.0, scale)?;
        method = m;
        sims.push((scale, similarity));
    }
    let success = sims.iter().all(|(_, s)| *s >= CROSS_RES_SIMILARITY);
    let record = EvidenceRecord {
        level: 4,
        procedure: RESOLUTION_PROCEDURE.into(),
        metrics: serde_json::json!({
            "success": success,
            "similarities": sims.iter().map(|(sc, s)| serde_json::json!({"scale": sc, "similarity": s})).collect::<Vec<_>>(),
            "threshold": CROSS_RES_SIMILARITY,
            "method": method,
        }),
        at: Utc::now(),
        node: node_name(),
    };
    apply(
        registry,
        id,
        record,
        if success { Some(4) } else { None },
        None,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    /// Small deterministic evolve in an isolated data dir; returns the
    /// data dir and a registered primitive id (with saved manifest).
    fn setup() -> (std::path::PathBuf, Registry, String) {
        let dir = std::env::temp_dir().join(format!("kc-evidence-{}", Uuid::new_v4()));
        std::env::set_var("KANNAKA_CRYSTAL_DATA_DIR", &dir);
        let cfg = EvolutionConfig {
            material_id: "metamaterial".into(),
            generations: 1,
            population: 3,
            field_size: 48,
            seed: 17,
            ..Default::default()
        };
        let mut registry = Registry::default();
        let report = evolve(&cfg, &mut registry, |_| {});
        report.manifest.save().unwrap();
        let id = registry
            .primitives
            .first()
            .expect("seeded discovery")
            .id
            .clone();
        (dir, registry, id)
    }

    #[test]
    fn reproduce_promotes_deterministic_discovery_to_level_2() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let (dir, mut registry, id) = setup();
        assert_eq!(registry.find(&id).unwrap().evidence_level, 1);
        let record = reproduce(&mut registry, &id).unwrap();
        assert_eq!(
            record.metrics["success"], true,
            "deterministic engine must replicate"
        );
        let prim = registry.find(&id).unwrap();
        assert_eq!(prim.evidence_level, 2, "replication promotes");
        assert_eq!(prim.evidence_records.len(), 1);
        std::env::remove_var("KANNAKA_CRYSTAL_DATA_DIR");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn reproduce_is_closed_under_registry_growth() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let (dir, mut registry, id) = setup();
        // Grow the registry with a second, different run. Full-evolution
        // replay diverges here (novelty is computed vs the live registry —
        // the exact failure observed on CRY-012627: similarity 0.861 after
        // its own run's 48 discoveries landed). Genome replay must not.
        let cfg2 = EvolutionConfig {
            material_id: "metamaterial".into(),
            generations: 1,
            population: 3,
            field_size: 48,
            seed: 99,
            ..Default::default()
        };
        let report2 = evolve(&cfg2, &mut registry, |_| {});
        report2.manifest.save().unwrap();
        let record = reproduce(&mut registry, &id).unwrap();
        assert_eq!(record.metrics["method"], "genome-replay");
        assert_eq!(
            record.metrics["success"], true,
            "genome replay is registry-independent: {}",
            record.metrics
        );
        assert_eq!(registry.find(&id).unwrap().evidence_level, 2);
        std::env::remove_var("KANNAKA_CRYSTAL_DATA_DIR");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn unmanifested_primitives_cannot_be_promoted() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let mut registry = Registry::default();
        // Fabricate a pre-Phase-1 style row: no experiment linkage.
        let (dir, seeded, id) = setup();
        let mut prim = seeded.find(&id).unwrap().clone();
        prim.experiment_id = None;
        prim.id = "CRY-999999".into();
        registry.primitives.push(prim);
        let err = reproduce(&mut registry, "CRY-999999").unwrap_err();
        assert!(err.contains("no experiment manifest"));
        let _ = seeded; // silence unused warnings on some toolchains
        std::env::remove_var("KANNAKA_CRYSTAL_DATA_DIR");
        let _ = std::fs::remove_dir_all(dir);
    }
}

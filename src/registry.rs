//! Crystal Registry — every discovered primitive gets a stable identity:
//! CRY-###### id, UUID, blake3 hash of its signature, properties, lineage.
//!
//! Persisted as JSON under the data dir (`KANNAKA_CRYSTAL_DATA_DIR`, default
//! `~/.kannaka-crystal/registry.json`).

use crate::primitives::{signature_similarity, DetectedStructure, PrimitiveClass};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Primitive {
    /// Human-facing id, e.g. `CRY-000145`.
    pub id: String,
    pub uuid: Uuid,
    /// blake3 of the quantized signature — content identity.
    pub hash: String,
    pub class: PrimitiveClass,
    /// Persistence in [0,1]: fraction of resonance retained over the
    /// discovery observation window.
    pub persistence: f64,
    /// Noise tolerance in [0,1]: persistence retained under injected noise.
    pub noise_tolerance: f64,
    pub stability_score: f64,
    pub energy_profile: Vec<f64>,
    pub material_id: String,
    pub centroid: (f64, f64),
    pub area: usize,
    pub signature: Vec<f64>,
    /// Parent primitive ids (structural genealogy).
    pub lineage: Vec<String>,
    pub discovered_at: DateTime<Utc>,
    /// Free-form provenance, e.g. "evolve gen 12" or "dream deep".
    pub provenance: String,
    /// ADR-0004 §6 experiment provenance: the manifest that produced this
    /// primitive. `None` on rows registered before manifests existed.
    #[serde(default)]
    pub experiment_id: Option<Uuid>,
    /// Protocol hash of that manifest — identical protocols hash
    /// identically, so reproductions are detectable.
    #[serde(default)]
    pub experiment_hash: Option<String>,
    /// ADR-0004 §5 classification metadata (classifier version,
    /// confidence, raw morphology features). `None` on pre-metadata rows.
    #[serde(default)]
    pub classification: Option<crate::primitives::Classification>,
    /// ADR-0004 §9 evidence ladder level (0 Generated … 8 Hardware
    /// Observed). Registration = Level 1 (Observed); promotion only
    /// through recorded procedures. Pre-ladder rows default to 1.
    #[serde(default = "default_evidence_level")]
    pub evidence_level: u8,
    /// The recorded procedures behind the level — the level is a summary
    /// of these records, and demotion is possible if one fails replication.
    #[serde(default)]
    pub evidence_records: Vec<EvidenceRecord>,
    /// ADR-0004 §7: the genome that produced this primitive (evolve runs).
    #[serde(default)]
    pub genome_id: Option<Uuid>,
    /// That genome's parents — genealogy as actual ancestry.
    #[serde(default)]
    pub parent_genome_ids: Vec<Uuid>,
    /// ADR-0004 §5 behavioral ontology (Phase 4): measured capability
    /// demonstrations. A capability is recorded whether it passed or not —
    /// failed tests are evidence too. Absent on pre-Phase-4 rows.
    #[serde(default)]
    pub behavioral_capabilities: Vec<BehavioralCapability>,
}

fn default_evidence_level() -> u8 {
    1
}

/// One recorded evidence procedure (ADR-0004 §9): what was run, what it
/// measured, when, and where.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRecord {
    /// The ladder level this record supports.
    pub level: u8,
    /// e.g. "reproduce-v1", "perturbation-ensemble-v1", "cross-resolution-v1".
    pub procedure: String,
    /// Procedure-specific metrics (similarities, survival rates, profiles).
    pub metrics: serde_json::Value,
    pub at: DateTime<Utc>,
    pub node: String,
}

/// One measured behavioral capability (ADR-0004 Phase 4): the same task
/// run with and without the primitive instantiated, over N trials.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralCapability {
    /// e.g. "noise_shielding", "pattern_completion".
    pub name: String,
    /// Contract version — bump when the trial protocol changes.
    pub contract_version: String,
    /// Whether the contract's pass criteria were met.
    pub passed: bool,
    /// Mean advantage of primitive-present over primitive-absent trials.
    pub mean_advantage: f64,
    pub std_advantage: f64,
    /// Fraction of trials with positive advantage.
    pub positive_fraction: f64,
    pub trials: u64,
    pub at: DateTime<Utc>,
    pub node: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Registry {
    pub primitives: Vec<Primitive>,
    pub next_serial: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Growth caps: (`bucket_cap`, `total_cap`), from
/// `KANNAKA_CRYSTAL_BUCKET_CAP` (default 150 per class×material) and
/// `KANNAKA_CRYSTAL_MAX_PRIMITIVES` (default 5000).
pub fn caps_from_env() -> (usize, usize) {
    let get = |key: &str, default: usize| {
        std::env::var(key)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    (
        get("KANNAKA_CRYSTAL_BUCKET_CAP", 150),
        get("KANNAKA_CRYSTAL_MAX_PRIMITIVES", 5000),
    )
}

pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("KANNAKA_CRYSTAL_DATA_DIR") {
        return PathBuf::from(dir);
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".kannaka-crystal")
}

fn registry_path() -> PathBuf {
    data_dir().join("registry.json")
}

impl Registry {
    pub fn load() -> Result<Self, RegistryError> {
        let path = registry_path();
        if !path.exists() {
            return Ok(Registry::default());
        }
        let text = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&text)?)
    }

    pub fn save(&self) -> Result<(), RegistryError> {
        let dir = data_dir();
        std::fs::create_dir_all(&dir)?;
        // Write-then-rename so a crash mid-save can't corrupt the registry.
        let tmp = dir.join("registry.json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(&tmp, registry_path())?;
        Ok(())
    }

    /// Highest signature similarity against any registered primitive.
    /// Novelty = 1 - this value.
    pub fn max_similarity(&self, signature: &[f64]) -> f64 {
        self.primitives
            .iter()
            .map(|p| signature_similarity(&p.signature, signature))
            .fold(0.0, f64::max)
    }

    /// Register a detected structure. Returns None if it duplicates an
    /// existing primitive (similarity >= 0.92 and same class).
    /// `experiment` links the ADR-0004 manifest (id, protocol hash) that
    /// produced this structure.
    #[allow(clippy::too_many_arguments)]
    pub fn register(
        &mut self,
        s: &DetectedStructure,
        persistence: f64,
        noise_tolerance: f64,
        energy_profile: Vec<f64>,
        material_id: &str,
        lineage: Vec<String>,
        provenance: &str,
        experiment: Option<(Uuid, String)>,
        genome: Option<(Uuid, Vec<Uuid>)>,
    ) -> Option<Primitive> {
        let duplicate = self.primitives.iter().any(|p| {
            p.class == s.class && signature_similarity(&p.signature, &s.signature) >= 0.92
        });
        if duplicate {
            return None;
        }
        self.next_serial += 1;
        let quantized: Vec<u8> = s.signature.iter().map(|v| (v * 255.0) as u8).collect();
        let (experiment_id, experiment_hash) = match experiment {
            Some((id, hash)) => (Some(id), Some(hash)),
            None => (None, None),
        };
        let prim = Primitive {
            id: format!("CRY-{:06}", self.next_serial),
            uuid: Uuid::new_v4(),
            hash: blake3::hash(&quantized).to_hex().to_string(),
            class: s.class,
            persistence,
            noise_tolerance,
            stability_score: s.stability_score,
            energy_profile,
            material_id: material_id.to_string(),
            centroid: s.centroid,
            area: s.area,
            signature: s.signature.clone(),
            lineage,
            discovered_at: Utc::now(),
            provenance: provenance.to_string(),
            experiment_id,
            experiment_hash,
            classification: Some(s.classification.clone()),
            // Level 1 Observed: passed the detector + registration gates.
            evidence_level: 1,
            evidence_records: Vec::new(),
            genome_id: genome.as_ref().map(|(id, _)| *id),
            parent_genome_ids: genome.map(|(_, parents)| parents).unwrap_or_default(),
            behavioral_capabilities: vec![],
        };
        self.primitives.push(prim.clone());
        Some(prim)
    }

    pub fn find(&self, id: &str) -> Option<&Primitive> {
        self.primitives
            .iter()
            .find(|p| p.id == id || p.uuid.to_string() == id)
    }

    pub fn find_mut(&mut self, id: &str) -> Option<&mut Primitive> {
        self.primitives
            .iter_mut()
            .find(|p| p.id == id || p.uuid.to_string() == id)
    }

    /// Merge a primitive announced by another swarm node (PRD v0.5).
    ///
    /// Identity across the swarm is the UUID (and near-duplicate structure
    /// is rejected exactly like local registration). Serial CRY ids are
    /// per-node, so the import gets a fresh local serial; the origin node's
    /// id is preserved in provenance. Lineage strings may reference serials
    /// that only resolve on the origin node — they are kept verbatim as a
    /// genealogical record, not a local foreign key.
    pub fn merge_remote(&mut self, remote: &Primitive, origin_node: &str) -> Option<String> {
        if self.primitives.iter().any(|p| p.uuid == remote.uuid) {
            return None;
        }
        let duplicate = self.primitives.iter().any(|p| {
            p.class == remote.class && signature_similarity(&p.signature, &remote.signature) >= 0.92
        });
        if duplicate {
            return None;
        }
        self.next_serial += 1;
        let mut prim = remote.clone();
        prim.id = format!("CRY-{:06}", self.next_serial);
        prim.provenance = format!("swarm:{origin_node}:{} | {}", remote.id, remote.provenance);
        let id = prim.id.clone();
        self.primitives.push(prim);
        Some(id)
    }

    /// Search the registry (PRD v0.5: "every primitive becomes searchable").
    /// `class` matches the display name case-insensitively with `_`/`-`
    /// treated as spaces (`standing_echo` == "Standing Echo").
    /// `min_evidence` filters by ADR-0004 §9 ladder level — the hook
    /// KannakaHDL's evidence-floor queries resolve through. `capability`
    /// requires a PASSED behavioral capability of that name (Phase 4) —
    /// a recorded-but-failed test does not satisfy the query.
    pub fn search(
        &self,
        class: Option<&str>,
        material: Option<&str>,
        min_persistence: f64,
        min_evidence: u8,
        capability: Option<&str>,
    ) -> Vec<&Primitive> {
        let normalize = |s: &str| s.to_lowercase().replace(['_', '-'], " ");
        self.primitives
            .iter()
            .filter(|p| {
                class.is_none_or(|c| normalize(&p.class.to_string()) == normalize(c))
                    && material.is_none_or(|m| p.material_id == m)
                    && p.persistence >= min_persistence
                    && p.evidence_level >= min_evidence
                    && capability.is_none_or(|cap| {
                        p.behavioral_capabilities
                            .iter()
                            .any(|c| c.passed && normalize(&c.name) == normalize(cap))
                    })
            })
            .collect()
    }

    /// Growth management (PRD: decay is information — it applies to the
    /// catalog too). Two bounds, both env-tunable:
    ///
    /// - `bucket_cap`: max primitives per (class, material) bucket, so one
    ///   prolific bucket (metamaterial Standing Echoes) can't squeeze out
    ///   taxonomy diversity.
    /// - `total_cap`: absolute registry size.
    ///
    /// Eviction order is lowest quality first, where quality is
    /// persistence-weighted (0.7) with noise tolerance (0.3). Lineage
    /// references may dangle after pruning — by ADR-0002 lineage is a
    /// genealogical record, not a foreign key. Returns evicted count.
    pub fn prune(&mut self, bucket_cap: usize, total_cap: usize) -> usize {
        use std::collections::HashMap;
        let quality = |p: &Primitive| p.persistence * 0.7 + p.noise_tolerance * 0.3;
        let before = self.primitives.len();

        // Per-bucket cap.
        let mut buckets: HashMap<(String, String), Vec<(f64, Uuid)>> = HashMap::new();
        for p in &self.primitives {
            buckets
                .entry((p.class.to_string(), p.material_id.clone()))
                .or_default()
                .push((quality(p), p.uuid));
        }
        let mut evict: std::collections::HashSet<Uuid> = std::collections::HashSet::new();
        for members in buckets.values_mut() {
            if members.len() > bucket_cap {
                members.sort_by(|a, b| b.0.total_cmp(&a.0));
                for (_, uuid) in members.iter().skip(bucket_cap) {
                    evict.insert(*uuid);
                }
            }
        }
        self.primitives.retain(|p| !evict.contains(&p.uuid));

        // Absolute cap.
        if self.primitives.len() > total_cap {
            self.primitives
                .sort_by(|a, b| quality(b).total_cmp(&quality(a)));
            self.primitives.truncate(total_cap);
        }
        before - self.primitives.len()
    }

    /// Similarity search: top-k most similar primitives to a signature.
    pub fn similar(&self, signature: &[f64], top_k: usize) -> Vec<(f64, &Primitive)> {
        let mut scored: Vec<(f64, &Primitive)> = self
            .primitives
            .iter()
            .map(|p| (signature_similarity(&p.signature, signature), p))
            .collect();
        scored.sort_by(|a, b| b.0.total_cmp(&a.0));
        scored.truncate(top_k);
        scored
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_structure(seed: f64) -> DetectedStructure {
        let mut signature = vec![0.0; 256];
        for (i, v) in signature.iter_mut().enumerate() {
            *v = ((i as f64 * seed).sin()).abs();
        }
        let norm: f64 = signature.iter().map(|v| v * v).sum::<f64>().sqrt();
        for v in signature.iter_mut() {
            *v /= norm;
        }
        DetectedStructure {
            class: PrimitiveClass::StandingEcho,
            classification: crate::primitives::Classification {
                display_class: "Standing Echo".into(),
                primitive_domain: "morphological".into(),
                classifier_version: crate::versions::CLASSIFIER_VERSION.into(),
                classifier_confidence: 0.8,
                features: crate::primitives::MorphologyFeatures {
                    relative_area: 0.01,
                    elongation: 1.2,
                    annularity: 0.1,
                    angular_gap_count: 0,
                    occupied_bins: 16,
                    stability_ratio: 2.5,
                },
            },
            centroid: (0.5, 0.5),
            area: 40,
            stability_score: 2.5,
            signature,
        }
    }

    #[test]
    fn register_assigns_sequential_ids_and_rejects_duplicates() {
        let mut r = Registry::default();
        let s1 = fake_structure(0.13);
        let experiment = Some((Uuid::new_v4(), "abc123".to_string()));
        let p1 = r
            .register(
                &s1,
                0.9,
                0.8,
                vec![1.0, 0.9],
                "silicon",
                vec![],
                "test",
                experiment.clone(),
                None,
            )
            .expect("first registration");
        assert_eq!(p1.id, "CRY-000001");
        // ADR-0004: experiment provenance + classification metadata land.
        assert_eq!(p1.experiment_id, experiment.as_ref().map(|(id, _)| *id));
        assert_eq!(p1.experiment_hash.as_deref(), Some("abc123"));
        let cls = p1.classification.expect("classification metadata");
        assert_eq!(cls.classifier_version, crate::versions::CLASSIFIER_VERSION);
        assert_eq!(cls.primitive_domain, "morphological");

        // Exact duplicate is rejected.
        assert!(r
            .register(&s1, 0.9, 0.8, vec![], "silicon", vec![], "test", None, None)
            .is_none());

        // A different structure registers with the next serial.
        let p2 = r
            .register(
                &fake_structure(0.77),
                0.5,
                0.4,
                vec![],
                "silicon",
                vec![],
                "test",
                None,
                None,
            )
            .expect("second registration");
        assert_eq!(p2.id, "CRY-000002");
        assert!(p2.experiment_id.is_none());
    }

    #[test]
    fn roundtrip_persistence() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("kc-test-{}", std::process::id()));
        std::env::set_var("KANNAKA_CRYSTAL_DATA_DIR", &dir);
        let mut r = Registry::default();
        r.register(
            &fake_structure(0.31),
            0.7,
            0.6,
            vec![2.0],
            "vacuum",
            vec![],
            "rt",
            None,
            None,
        );
        r.save().unwrap();
        let loaded = Registry::load().unwrap();
        assert_eq!(loaded.primitives.len(), 1);
        assert_eq!(loaded.primitives[0].material_id, "vacuum");
        std::env::remove_var("KANNAKA_CRYSTAL_DATA_DIR");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn prune_respects_bucket_and_total_caps_keeping_quality() {
        let mut r = Registry::default();
        for i in 0..30 {
            let s = fake_structure(0.1 + i as f64 * 0.037);
            // Rising persistence so the best are the later ones.
            r.register(
                &s,
                0.2 + (i as f64) * 0.02,
                0.5,
                vec![],
                "silicon",
                vec![],
                "t",
                None,
                None,
            );
        }
        assert!(r.primitives.len() > 10, "need a populated registry");
        let mut persistences: Vec<f64> = r.primitives.iter().map(|p| p.persistence).collect();
        persistences.sort_by(|a, b| b.total_cmp(a));
        let tenth_best = persistences[9];

        let evicted = r.prune(10, 100);
        assert_eq!(r.primitives.len(), 10, "bucket cap enforced");
        assert!(evicted > 0);
        assert!(
            r.primitives
                .iter()
                .all(|p| p.persistence >= tenth_best - 1e-9),
            "prune must keep highest quality"
        );

        // Absolute cap dominates.
        let evicted2 = r.prune(10, 4);
        assert_eq!(r.primitives.len(), 4);
        assert_eq!(evicted2, 6);
    }

    #[test]
    fn merge_remote_imports_once_and_reserials() {
        let mut local = Registry::default();
        let mut origin = Registry::default();
        let remote = origin
            .register(
                &fake_structure(0.13),
                0.9,
                0.8,
                vec![],
                "silicon",
                vec![],
                "evolve gen 1",
                None,
                None,
            )
            .unwrap();

        let merged_id = local
            .merge_remote(&remote, "crystal-abc123")
            .expect("first merge");
        assert_eq!(merged_id, "CRY-000001", "local serial, not origin's");
        let merged = local.find(&merged_id).unwrap();
        assert_eq!(merged.uuid, remote.uuid, "swarm identity is the uuid");
        assert!(merged
            .provenance
            .contains("swarm:crystal-abc123:CRY-000001"));

        // Re-announcement is idempotent; near-duplicate structure rejected.
        assert!(local.merge_remote(&remote, "crystal-abc123").is_none());
        let near_dup = origin.register(
            &fake_structure(0.13001),
            0.9,
            0.8,
            vec![],
            "silicon",
            vec![],
            "t",
            None,
            None,
        );
        if let Some(nd) = near_dup {
            assert!(local.merge_remote(&nd, "crystal-def456").is_none());
        }
    }

    #[test]
    fn search_filters_class_material_persistence() {
        let mut r = Registry::default();
        r.register(
            &fake_structure(0.13),
            0.9,
            0.8,
            vec![],
            "silicon",
            vec![],
            "t",
            None,
            None,
        );
        r.register(
            &fake_structure(0.77),
            0.3,
            0.4,
            vec![],
            "vacuum",
            vec![],
            "t",
            None,
            None,
        );
        assert_eq!(r.search(Some("standing_echo"), None, 0.0, 0, None).len(), 2);
        assert_eq!(
            r.search(Some("Standing Echo"), Some("silicon"), 0.0, 0, None)
                .len(),
            1
        );
        assert_eq!(r.search(None, None, 0.5, 0, None).len(), 1);
        assert_eq!(r.search(Some("echo ring"), None, 0.0, 0, None).len(), 0);
    }

    #[test]
    fn similarity_search_ranks_self_first() {
        let mut r = Registry::default();
        let a = fake_structure(0.13);
        let b = fake_structure(0.99);
        r.register(&a, 0.9, 0.8, vec![], "silicon", vec![], "t", None, None);
        r.register(&b, 0.9, 0.8, vec![], "silicon", vec![], "t", None, None);
        let hits = r.similar(&a.signature, 2);
        assert!(hits[0].0 > 0.999);
    }
}

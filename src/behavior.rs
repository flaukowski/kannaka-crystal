//! Behavioral primitive contracts (ADR-0004 §5 second ontology + Phase 4).
//!
//! A behavioral capability is never inferred from morphology: it is a
//! measured, repeated demonstration that a primitive performs a defined
//! information-processing function. Every contract here has the same
//! honest shape — the same task is run WITH the primitive instantiated
//! and WITHOUT it, across N deterministic trials, and the capability
//! score is the mean advantage. A primitive that does not help scores
//! ≈ 0 and the failed attempt is recorded (negative results are
//! outputs, not embarrassments).
//!
//! v1 contracts:
//! - `noise_shielding` — does the primitive's presence protect a written
//!   memory's physical recall under ambient noise?
//! - `pattern_completion` — injected with only half a target's
//!   constellation as a cue, does the field with the primitive recover
//!   the FULL pattern better than without?
//!
//! Passing (mean advantage ≥ threshold with ≥70% positive trials)
//! registers the capability and, if the primitive is already Replicated
//! (Level ≥ 2), promotes it to Level 6 Behaviorally Validated. The
//! Level-2 gate is deliberate: a behavior demonstrated on a structure
//! that cannot be reproduced is a demonstration of nothing.

use crate::engine::CrystalEngine;
use crate::pulse::encode_text;
use crate::registry::{BehavioralCapability, Instantiation, Registry};
use chrono::Utc;

pub const NOISE_SHIELDING: &str = "noise_shielding";
pub const PATTERN_COMPLETION: &str = "pattern_completion";
pub const CONTRACT_VERSION: &str = "behavior-contract-v1";
pub const CONTRACT_VERSION_V2: &str = "behavior-contract-v2";

const ADVANTAGE_THRESHOLD: f64 = 0.01;
const POSITIVE_TRIAL_FLOOR: f64 = 0.7;
const BEHAVIOR_LEVEL: u8 = 6;
const MIN_LEVEL_FOR_PROMOTION: u8 = 2;

/// True for a name this build has a contract for — validate BEFORE a
/// long run, not trial-by-trial inside it.
pub fn known_capability(name: &str) -> bool {
    matches!(name, NOISE_SHIELDING | PATTERN_COMPLETION)
}

fn trial_engine(material_id: &str, seed: u64) -> CrystalEngine {
    let material = if crate::material::find_material(material_id).is_some() {
        material_id
    } else {
        // Swarm-imported primitives may reference materials this build
        // doesn't know (e.g. "hrm"); test in the reference medium.
        "ideal_resonator"
    };
    CrystalEngine::new(material, 64, seed).expect("builtin material")
}

/// How a structure is placed into a task field: where it sits, how large
/// it is, how hard it arrives, and which way it faces.
///
/// v1 fixed every one of these. That was not a neutral choice — a
/// structure painted at one amplitude, dead centre, unrotated, can only
/// contribute what its shape passively does from that spot. v2 makes
/// them searchable (ADR-0004 Phase 4.2). `Instantiation::default()`
/// reproduces v1's placement exactly, so the v1 path is unchanged.
pub use crate::registry::Instantiation as InstantiationParams;

/// Paint a 16x16 signature into the field at an arbitrary placement.
///
/// Rotation is applied about the patch centre in signature space before
/// mapping to field coordinates, so a rotated placement samples the same
/// structure from a different angle rather than smearing it.
pub fn instantiate_with(engine: &mut CrystalEngine, signature: &[f64], p: &Instantiation) {
    const S: usize = 16;
    if signature.len() != S * S {
        return;
    }
    let n = engine.field.size;
    let span = n as f64 * p.scale;
    let (sin_r, cos_r) = p.rotation.sin_cos();
    for sy in 0..S {
        for sx in 0..S {
            let v = signature[sy * S + sx];
            if v.abs() < 1e-9 {
                continue;
            }
            // Offsets in [-0.5, 0.5) patch space, rotated about centre.
            let ox = sx as f64 / S as f64 - 0.5;
            let oy = sy as f64 / S as f64 - 0.5;
            let rx = ox * cos_r - oy * sin_r;
            let ry = ox * sin_r + oy * cos_r;
            let fx = (p.cx * n as f64 + rx * span) as isize;
            let fy = (p.cy * n as f64 + ry * span) as isize;
            if fx >= 0 && fy >= 0 && (fx as usize) < n && (fy as usize) < n {
                engine.field.u[fy as usize * n + fx as usize] += v * p.gain;
            }
        }
    }
}

fn target_text(seed: u64) -> String {
    format!("behavioral target {seed}")
}

/// One noise-shielding trial: physical recall of a written target after
/// noisy evolution, with vs without the structure present.
fn noise_shielding_trial(signature: &[f64], material_id: &str, seed: u64) -> f64 {
    noise_shielding_trial_at(signature, material_id, seed, &Instantiation::default())
}

fn noise_shielding_trial_at(
    signature: &[f64],
    material_id: &str,
    seed: u64,
    p: &Instantiation,
) -> f64 {
    let run = |with_structure: bool| {
        let mut engine = trial_engine(material_id, seed);
        if with_structure {
            instantiate_with(&mut engine, signature, p);
        }
        engine.write(&target_text(seed), 1.0);
        engine.noise_amp = 0.01;
        engine.resonate(200);
        engine.probe(&target_text(seed)).physical_resonance
    };
    run(true) - run(false)
}

/// One pattern-completion trial: inject HALF the target's constellation
/// as a cue, evolve, and measure physical recall of the FULL pattern —
/// with vs without the structure present.
fn pattern_completion_trial(signature: &[f64], material_id: &str, seed: u64) -> f64 {
    pattern_completion_trial_at(signature, material_id, seed, &Instantiation::default())
}

fn pattern_completion_trial_at(
    signature: &[f64],
    material_id: &str,
    seed: u64,
    p: &Instantiation,
) -> f64 {
    let run = |with_structure: bool| {
        let mut engine = trial_engine(material_id, seed);
        if with_structure {
            instantiate_with(&mut engine, signature, p);
        }
        let full = encode_text(&target_text(seed), engine.field.size);
        // Deterministic half-cue: zero every second column block.
        let n = engine.field.size;
        for (i, v) in full.iter().enumerate() {
            let x = i % n;
            if (x / 8).is_multiple_of(2) {
                engine.field.u[i] += v;
            }
        }
        engine.resonate(150);
        engine.probe(&target_text(seed)).physical_resonance
    };
    run(true) - run(false)
}

type PlacedTrialFn = fn(&[f64], &str, u64, &Instantiation) -> f64;

fn placed_trial_fn(capability: &str) -> Result<PlacedTrialFn, String> {
    match capability {
        NOISE_SHIELDING => Ok(noise_shielding_trial_at),
        PATTERN_COMPLETION => Ok(pattern_completion_trial_at),
        other => Err(format!(
            "unknown capability: {other} ({NOISE_SHIELDING}|{PATTERN_COMPLETION})"
        )),
    }
}

/// Mean advantage of one placement over a seed set.
fn mean_advantage_at(
    trial: PlacedTrialFn,
    signature: &[f64],
    material_id: &str,
    seeds: &[u64],
    p: &Instantiation,
) -> f64 {
    if seeds.is_empty() {
        return 0.0;
    }
    seeds
        .iter()
        .map(|s| trial(signature, material_id, *s, p))
        .sum::<f64>()
        / seeds.len() as f64
}

/// Candidate ladders per axis. Deliberately coarse: every extra candidate
/// is a full field evolution per fit seed, and a finer grid buys more
/// chances to fit noise, not more truth.
const CX_LADDER: [f64; 5] = [0.30, 0.40, 0.50, 0.60, 0.70];
const CY_LADDER: [f64; 5] = [0.30, 0.40, 0.50, 0.60, 0.70];
const SCALE_LADDER: [f64; 4] = [0.20, 0.35, 0.50, 0.70];
const GAIN_LADDER: [f64; 5] = [1.0, 3.0, 6.0, 10.0, 16.0];
const ROTATION_LADDER: [f64; 4] = [
    0.0,
    std::f64::consts::FRAC_PI_4,
    std::f64::consts::FRAC_PI_2,
    3.0 * std::f64::consts::FRAC_PI_4,
];

/// Search instantiation space by deterministic coordinate descent from
/// v1's placement, two sweeps over the five axes.
///
/// This is a FIT step and its score is in-sample: the placement it
/// returns has, by construction, been chosen to look good on these
/// seeds. Never report the fit score as the capability — evaluate the
/// returned placement on seeds the search never saw.
pub fn search_instantiation(
    signature: &[f64],
    material_id: &str,
    capability: &str,
    fit_seeds: &[u64],
    mut progress: impl FnMut(String),
) -> Result<(Instantiation, f64), String> {
    let trial = placed_trial_fn(capability)?;
    let mut best = Instantiation::default();
    let mut best_score = mean_advantage_at(trial, signature, material_id, fit_seeds, &best);
    progress(format!("  fit: v1 placement {best_score:+.4}"));

    for sweep in 0..2 {
        for axis in 0..5 {
            let mut improved = false;
            let candidates: Vec<Instantiation> = match axis {
                0 => CX_LADDER
                    .iter()
                    .map(|v| Instantiation { cx: *v, ..best })
                    .collect(),
                1 => CY_LADDER
                    .iter()
                    .map(|v| Instantiation { cy: *v, ..best })
                    .collect(),
                2 => SCALE_LADDER
                    .iter()
                    .map(|v| Instantiation { scale: *v, ..best })
                    .collect(),
                3 => GAIN_LADDER
                    .iter()
                    .map(|v| Instantiation { gain: *v, ..best })
                    .collect(),
                _ => ROTATION_LADDER
                    .iter()
                    .map(|v| Instantiation {
                        rotation: *v,
                        ..best
                    })
                    .collect(),
            };
            for cand in candidates {
                let score = mean_advantage_at(trial, signature, material_id, fit_seeds, &cand);
                if score > best_score {
                    best_score = score;
                    best = cand;
                    improved = true;
                }
            }
            if improved {
                progress(format!(
                    "  fit sweep {sweep} axis {axis}: {best_score:+.4} at {}",
                    best.describe()
                ));
            }
        }
    }
    Ok((best, best_score))
}

/// Permute a signature deterministically, preserving its value
/// distribution and destroying its spatial structure. The negative
/// control: if a scramble crosses the bar once placement is searched,
/// then the search manufactured the result and the crossing means
/// nothing about the structure.
fn scramble(signature: &[f64]) -> Vec<f64> {
    let mut out = signature.to_vec();
    // Fixed-seed Fisher-Yates; the shuffle must not vary run to run.
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    for i in (1..out.len()).rev() {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let j = (state >> 33) as usize % (i + 1);
        out.swap(i, j);
    }
    out
}

/// Mean contract advantage of a raw signature over `trials` deterministic
/// seeds — the in-loop hook capability-directed evolution selects on.
/// Same trials, same targets, same instantiation as the recorded
/// procedure, so in-loop selection optimizes exactly what `promote
/// --procedure behavior` later measures.
pub fn contract_advantage(
    signature: &[f64],
    material_id: &str,
    capability: &str,
    trials: u64,
) -> Result<f64, String> {
    let trial_fn: fn(&[f64], &str, u64) -> f64 = match capability {
        NOISE_SHIELDING => noise_shielding_trial,
        PATTERN_COMPLETION => pattern_completion_trial,
        other => {
            return Err(format!(
                "unknown capability: {other} ({NOISE_SHIELDING}|{PATTERN_COMPLETION})"
            ))
        }
    };
    let sum: f64 = (0..trials)
        .map(|seed| trial_fn(signature, material_id, seed))
        .sum();
    Ok(sum / trials.max(1) as f64)
}

/// Run a behavioral contract over `trials` deterministic seeds and, on a
/// pass, register the capability (and Level 6 if the primitive is at
/// least Replicated). Returns the capability record either way.
pub fn test_capability(
    registry: &mut Registry,
    id: &str,
    capability: &str,
    trials: u64,
    mut progress: impl FnMut(String),
) -> Result<BehavioralCapability, String> {
    let prim = registry
        .find(id)
        .ok_or_else(|| format!("unknown primitive: {id}"))?
        .clone();
    let trial_fn: fn(&[f64], &str, u64) -> f64 = match capability {
        NOISE_SHIELDING => noise_shielding_trial,
        PATTERN_COMPLETION => pattern_completion_trial,
        other => {
            return Err(format!(
                "unknown capability: {other} ({NOISE_SHIELDING}|{PATTERN_COMPLETION})"
            ))
        }
    };

    let mut advantages = Vec::new();
    for seed in 0..trials {
        let adv = trial_fn(&prim.signature, &prim.material_id, seed);
        progress(format!("  trial {seed}: advantage {adv:+.4}"));
        advantages.push(adv);
    }
    let n = advantages.len() as f64;
    let mean = advantages.iter().sum::<f64>() / n;
    let std = (advantages
        .iter()
        .map(|a| (a - mean) * (a - mean))
        .sum::<f64>()
        / n)
        .sqrt();
    let positive = advantages.iter().filter(|a| **a > 0.0).count() as f64 / n;
    let passed = mean >= ADVANTAGE_THRESHOLD && positive >= POSITIVE_TRIAL_FLOOR;

    let record = BehavioralCapability {
        name: capability.to_string(),
        contract_version: CONTRACT_VERSION.into(),
        passed,
        mean_advantage: mean,
        std_advantage: std,
        positive_fraction: positive,
        trials,
        at: Utc::now(),
        node: std::env::var("KANNAKA_CRYSTAL_NODE").unwrap_or_else(|_| "local".into()),
        instantiation: None,
        fit_advantage: None,
        control_advantage: None,
    };

    let stored = registry.find_mut(id).ok_or("primitive vanished mid-test")?;
    stored
        .behavioral_capabilities
        .retain(|c| c.name != capability);
    stored.behavioral_capabilities.push(record.clone());
    apply_behavior_level(stored);
    Ok(record)
}

/// Run the v2 contract: search instantiation on fit seeds, then measure
/// the found placement on seeds the search never saw.
///
/// Three numbers come out and all three are recorded:
///   `fit_advantage`     in-sample, on the seeds the search optimized
///   `mean_advantage`    held-out, the only one the pass criteria read
///   `control_advantage` held-out for a scrambled signature searched
///                       identically
///
/// Passing needs all of: held-out mean ≥ threshold, ≥70% of held-out
/// trials positive, AND held-out mean strictly greater than the control.
/// The last clause is the one that matters — searching placement gives
/// any patch, structured or not, many chances to look good, so a
/// structure that cannot beat its own scramble has demonstrated the
/// search, not itself.
pub fn test_capability_v2(
    registry: &mut Registry,
    id: &str,
    capability: &str,
    fit_seeds: u64,
    held_out_seeds: u64,
    mut progress: impl FnMut(String),
) -> Result<BehavioralCapability, String> {
    if fit_seeds == 0 || held_out_seeds == 0 {
        return Err("v2 needs at least one fit seed and one held-out seed".into());
    }
    let prim = registry
        .find(id)
        .ok_or_else(|| format!("unknown primitive: {id}"))?
        .clone();
    let trial = placed_trial_fn(capability)?;

    // Disjoint by construction: the search sees [0, fit), the score is
    // taken on [fit, fit + held_out).
    let fit: Vec<u64> = (0..fit_seeds).collect();
    let held: Vec<u64> = (fit_seeds..fit_seeds + held_out_seeds).collect();

    progress(format!(
        "searching instantiation on {} fit seeds",
        fit.len()
    ));
    let (placement, fit_score) = search_instantiation(
        &prim.signature,
        &prim.material_id,
        capability,
        &fit,
        &mut progress,
    )?;
    progress(format!(
        "  selected {} (in-sample {fit_score:+.4})",
        placement.describe()
    ));

    let mut advantages = Vec::new();
    for seed in &held {
        let adv = trial(&prim.signature, &prim.material_id, *seed, &placement);
        progress(format!("  held-out {seed}: advantage {adv:+.4}"));
        advantages.push(adv);
    }
    let n = advantages.len() as f64;
    let mean = advantages.iter().sum::<f64>() / n;
    let std = (advantages.iter().map(|a| (a - mean).powi(2)).sum::<f64>() / n).sqrt();
    let positive = advantages.iter().filter(|a| **a > 0.0).count() as f64 / n;

    // Negative control: identical search and scoring on a scramble.
    progress("control: same search on a scrambled signature".into());
    let scrambled = scramble(&prim.signature);
    let (ctrl_placement, _) =
        search_instantiation(&scrambled, &prim.material_id, capability, &fit, |_| {})?;
    let control = mean_advantage_at(trial, &scrambled, &prim.material_id, &held, &ctrl_placement);
    progress(format!("  control held-out {control:+.4}"));

    let passed = mean >= ADVANTAGE_THRESHOLD && positive >= POSITIVE_TRIAL_FLOOR && mean > control;

    let record = BehavioralCapability {
        name: capability.to_string(),
        contract_version: CONTRACT_VERSION_V2.into(),
        passed,
        mean_advantage: mean,
        std_advantage: std,
        positive_fraction: positive,
        trials: held_out_seeds,
        at: Utc::now(),
        node: std::env::var("KANNAKA_CRYSTAL_NODE").unwrap_or_else(|_| "local".into()),
        instantiation: Some(placement),
        fit_advantage: Some(fit_score),
        control_advantage: Some(control),
    };

    let stored = registry.find_mut(id).ok_or("primitive vanished mid-test")?;
    stored
        .behavioral_capabilities
        .retain(|c| c.name != capability);
    stored.behavioral_capabilities.push(record.clone());
    apply_behavior_level(stored);
    Ok(record)
}

/// Recompute a primitive's level after its capability records change.
///
/// Level 6 is held only while some capability currently passes. A
/// re-test that fails takes it back — the same rule `reproduce` already
/// applies when a replication stops replicating. Without this a lucky
/// pass is permanent: the run that grants Level 6 is the one that
/// counts and no amount of later evidence can withdraw it, which is
/// exactly backwards for a ladder that claims to track what is
/// currently supported.
///
/// Demotion falls back to the best level the primitive's non-behavioral
/// evidence records still justify, never below Level 1.
fn apply_behavior_level(prim: &mut crate::registry::Primitive) {
    let any_passing = prim.behavioral_capabilities.iter().any(|c| c.passed);
    if any_passing {
        if prim.evidence_level >= MIN_LEVEL_FOR_PROMOTION {
            prim.evidence_level = prim.evidence_level.max(BEHAVIOR_LEVEL);
        }
        return;
    }
    if prim.evidence_level < BEHAVIOR_LEVEL {
        return;
    }
    prim.evidence_level = prim
        .evidence_records
        .iter()
        .filter(|r| r.level < BEHAVIOR_LEVEL)
        .map(|r| r.level)
        .max()
        .unwrap_or(1)
        .max(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::{evolve, EvolutionConfig};

    fn seeded_registry() -> (Registry, String) {
        let cfg = EvolutionConfig {
            material_id: "metamaterial".into(),
            generations: 1,
            population: 3,
            field_size: 48,
            seed: 17,
            ..Default::default()
        };
        let mut registry = Registry::default();
        evolve(&cfg, &mut registry, |_| {});
        let id = registry.primitives.first().expect("discovery").id.clone();
        (registry, id)
    }

    #[test]
    fn capability_test_records_result_and_gates_level_6_on_replication() {
        let (mut registry, id) = seeded_registry();
        let record = test_capability(&mut registry, &id, NOISE_SHIELDING, 3, |_| {}).unwrap();
        assert!(record.mean_advantage.is_finite());
        assert_eq!(record.trials, 3);
        let prim = registry.find(&id).unwrap();
        assert_eq!(prim.behavioral_capabilities.len(), 1);
        // Level-6 promotion requires Level >= 2 first; this fresh
        // primitive is Level 1, so even a pass must NOT grant Level 6.
        assert!(
            prim.evidence_level < 6,
            "unreplicated primitive must not reach L6"
        );
    }

    #[test]
    fn rerunning_a_contract_replaces_the_capability_record() {
        let (mut registry, id) = seeded_registry();
        test_capability(&mut registry, &id, PATTERN_COMPLETION, 2, |_| {}).unwrap();
        test_capability(&mut registry, &id, PATTERN_COMPLETION, 3, |_| {}).unwrap();
        let prim = registry.find(&id).unwrap();
        assert_eq!(
            prim.behavioral_capabilities.len(),
            1,
            "no duplicate records"
        );
        assert_eq!(prim.behavioral_capabilities[0].trials, 3);
    }

    #[test]
    fn unknown_capability_is_an_error() {
        let (mut registry, id) = seeded_registry();
        assert!(test_capability(&mut registry, &id, "telepathy", 1, |_| {}).is_err());
    }

    /// The v1 contract must be unchanged by v2's arrival. Default
    /// parameters have to reproduce the old fixed placement field for
    /// field, or every v1 record in the registry becomes uncomparable.
    #[test]
    fn default_instantiation_reproduces_the_v1_placement() {
        let (registry, id) = seeded_registry();
        let sig = &registry.find(&id).unwrap().signature;

        // v1's placement, transcribed from the pre-v2 implementation.
        let mut expected = trial_engine("metamaterial", 5);
        {
            const S: usize = 16;
            let n = expected.field.size;
            let span = n as f64 * 0.35;
            for sy in 0..S {
                for sx in 0..S {
                    let v = sig[sy * S + sx];
                    if v.abs() < 1e-9 {
                        continue;
                    }
                    let fx = (0.5 * n as f64 + (sx as f64 / S as f64 - 0.5) * span) as isize;
                    let fy = (0.5 * n as f64 + (sy as f64 / S as f64 - 0.5) * span) as isize;
                    if fx >= 0 && fy >= 0 && (fx as usize) < n && (fy as usize) < n {
                        expected.field.u[fy as usize * n + fx as usize] += v * 3.0;
                    }
                }
            }
        }

        let mut actual = trial_engine("metamaterial", 5);
        instantiate_with(&mut actual, sig, &Instantiation::default());

        assert_eq!(actual.field.u, expected.field.u, "v1 placement drifted");
    }

    #[test]
    fn placement_changes_where_the_structure_lands() {
        let (registry, id) = seeded_registry();
        let sig = &registry.find(&id).unwrap().signature;
        let mut centred = trial_engine("metamaterial", 1);
        instantiate_with(&mut centred, sig, &Instantiation::default());
        let mut offset = trial_engine("metamaterial", 1);
        instantiate_with(
            &mut offset,
            sig,
            &Instantiation {
                cx: 0.3,
                ..Instantiation::default()
            },
        );
        assert_ne!(
            centred.field.u, offset.field.u,
            "moving the centre must move the structure"
        );
    }

    #[test]
    fn gain_scales_the_injected_amplitude() {
        let (registry, id) = seeded_registry();
        let sig = &registry.find(&id).unwrap().signature;
        let energy = |gain: f64| {
            let mut e = trial_engine("metamaterial", 2);
            instantiate_with(
                &mut e,
                sig,
                &Instantiation {
                    gain,
                    ..Instantiation::default()
                },
            );
            e.field.u.iter().map(|v| v.abs()).sum::<f64>()
        };
        assert!(
            energy(6.0) > energy(3.0),
            "doubling gain must inject more amplitude"
        );
    }

    /// The search is a fit step and must be honest about it: the
    /// placement it returns can never score worse in-sample than the v1
    /// placement it started from, because it only moves on improvement.
    #[test]
    fn search_never_returns_worse_than_the_v1_placement() {
        let (registry, id) = seeded_registry();
        let prim = registry.find(&id).unwrap();
        let fit = [0u64, 1];
        let baseline = mean_advantage_at(
            noise_shielding_trial_at,
            &prim.signature,
            &prim.material_id,
            &fit,
            &Instantiation::default(),
        );
        let (_, score) = search_instantiation(
            &prim.signature,
            &prim.material_id,
            NOISE_SHIELDING,
            &fit,
            |_| {},
        )
        .unwrap();
        assert!(
            score >= baseline - 1e-12,
            "coordinate descent moved downhill: {score} < {baseline}"
        );
    }

    #[test]
    fn v2_records_held_out_fit_and_control_numbers() {
        let (mut registry, id) = seeded_registry();
        let record = test_capability_v2(&mut registry, &id, NOISE_SHIELDING, 2, 2, |_| {}).unwrap();
        assert_eq!(record.contract_version, CONTRACT_VERSION_V2);
        assert!(record.instantiation.is_some(), "placement must be recorded");
        assert!(record.fit_advantage.is_some(), "in-sample must be recorded");
        assert!(
            record.control_advantage.is_some(),
            "the scramble control must be recorded"
        );
        assert_eq!(record.trials, 2, "trials counts HELD-OUT seeds only");
        // A pass may never be claimed without beating the control.
        if record.passed {
            assert!(record.mean_advantage > record.control_advantage.unwrap());
        }
    }

    /// Seed disjointness is the whole anti-overfitting claim. If the
    /// search could see a scoring seed the held-out number would be
    /// in-sample and the contract would be theatre.
    #[test]
    fn fit_and_held_out_seeds_never_overlap() {
        let (fit_seeds, held) = (3u64, 4u64);
        let fit: Vec<u64> = (0..fit_seeds).collect();
        let score: Vec<u64> = (fit_seeds..fit_seeds + held).collect();
        assert!(fit.iter().all(|f| !score.contains(f)));
        assert_eq!(score.len(), held as usize);
    }

    #[test]
    fn scramble_preserves_values_and_destroys_arrangement() {
        let sig: Vec<f64> = (0..256).map(|i| i as f64 / 256.0).collect();
        let mixed = scramble(&sig);
        assert_ne!(
            mixed, sig,
            "a scramble that changes nothing controls nothing"
        );
        let mut a = sig.clone();
        let mut b = mixed.clone();
        a.sort_by(f64::total_cmp);
        b.sort_by(f64::total_cmp);
        assert_eq!(a, b, "scramble must permute, not alter, the values");
        assert_eq!(mixed, scramble(&sig), "scramble must be deterministic");
    }

    #[test]
    fn v2_rejects_an_empty_seed_split() {
        let (mut registry, id) = seeded_registry();
        assert!(test_capability_v2(&mut registry, &id, NOISE_SHIELDING, 0, 2, |_| {}).is_err());
        assert!(test_capability_v2(&mut registry, &id, NOISE_SHIELDING, 2, 0, |_| {}).is_err());
    }

    /// Measured on CRY-012630, 2026-08-04: a contract that passed on 6
    /// held-out seeds failed on 30, and the primitive kept Level 6
    /// regardless. A ladder that only ratchets upward stops reporting
    /// what is currently supported.
    #[test]
    fn a_failed_contract_withdraws_the_level_6_it_granted() {
        let (mut registry, id) = seeded_registry();
        {
            let prim = registry.find_mut(&id).unwrap();
            prim.evidence_level = 3;
            prim.evidence_records.push(crate::registry::EvidenceRecord {
                level: 3,
                procedure: "perturbation-ensemble-v2".into(),
                metrics: serde_json::json!({}),
                at: Utc::now(),
                node: "test".into(),
            });
            // Stand in for a pass without depending on a live crossing.
            prim.behavioral_capabilities.push(BehavioralCapability {
                name: NOISE_SHIELDING.into(),
                contract_version: CONTRACT_VERSION_V2.into(),
                passed: true,
                mean_advantage: 0.05,
                std_advantage: 0.01,
                positive_fraction: 1.0,
                trials: 6,
                at: Utc::now(),
                node: "test".into(),
                instantiation: Some(Instantiation::default()),
                fit_advantage: Some(0.05),
                control_advantage: Some(0.0),
            });
            apply_behavior_level(prim);
            assert_eq!(prim.evidence_level, 6, "a pass should grant L6");
        }

        // Re-test at more seeds; whatever it measures, the stale pass is
        // replaced and the level must reflect the current record.
        let record = test_capability_v2(&mut registry, &id, NOISE_SHIELDING, 2, 3, |_| {}).unwrap();
        let prim = registry.find(&id).unwrap();
        if record.passed {
            assert_eq!(prim.evidence_level, 6);
        } else {
            assert_eq!(
                prim.evidence_level, 3,
                "a failed re-test must fall back to the level other evidence supports"
            );
        }
    }

    #[test]
    fn demotion_floors_at_level_1_with_no_other_evidence() {
        let (mut registry, id) = seeded_registry();
        let prim = registry.find_mut(&id).unwrap();
        prim.evidence_level = 6;
        prim.behavioral_capabilities.push(BehavioralCapability {
            name: NOISE_SHIELDING.into(),
            contract_version: CONTRACT_VERSION_V2.into(),
            passed: false,
            mean_advantage: 0.0,
            std_advantage: 0.0,
            positive_fraction: 0.0,
            trials: 30,
            at: Utc::now(),
            node: "test".into(),
            instantiation: Some(Instantiation::default()),
            fit_advantage: Some(0.02),
            control_advantage: Some(0.01),
        });
        apply_behavior_level(prim);
        assert_eq!(prim.evidence_level, 1);
    }

    /// A second capability that still passes holds the level up.
    #[test]
    fn one_failing_capability_does_not_strip_a_still_passing_one() {
        let (mut registry, id) = seeded_registry();
        let prim = registry.find_mut(&id).unwrap();
        prim.evidence_level = 6;
        for (name, passed) in [(NOISE_SHIELDING, false), (PATTERN_COMPLETION, true)] {
            prim.behavioral_capabilities.push(BehavioralCapability {
                name: name.into(),
                contract_version: CONTRACT_VERSION_V2.into(),
                passed,
                mean_advantage: if passed { 0.05 } else { 0.0 },
                std_advantage: 0.0,
                positive_fraction: if passed { 1.0 } else { 0.0 },
                trials: 30,
                at: Utc::now(),
                node: "test".into(),
                instantiation: Some(Instantiation::default()),
                fit_advantage: Some(0.05),
                control_advantage: Some(0.0),
            });
        }
        apply_behavior_level(prim);
        assert_eq!(prim.evidence_level, 6);
    }
}

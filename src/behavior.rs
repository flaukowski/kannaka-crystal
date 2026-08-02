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
use crate::registry::{BehavioralCapability, Primitive, Registry};
use chrono::Utc;

pub const NOISE_SHIELDING: &str = "noise_shielding";
pub const PATTERN_COMPLETION: &str = "pattern_completion";
pub const CONTRACT_VERSION: &str = "behavior-contract-v1";

const ADVANTAGE_THRESHOLD: f64 = 0.01;
const POSITIVE_TRIAL_FLOOR: f64 = 0.7;
const BEHAVIOR_LEVEL: u8 = 6;
const MIN_LEVEL_FOR_PROMOTION: u8 = 2;

fn trial_engine(prim: &Primitive, seed: u64) -> CrystalEngine {
    let material = if crate::material::find_material(&prim.material_id).is_some() {
        prim.material_id.clone()
    } else {
        // Swarm-imported primitives may reference materials this build
        // doesn't know (e.g. "hrm"); test in the reference medium.
        "ideal_resonator".to_string()
    };
    CrystalEngine::new(&material, 64, seed).expect("builtin material")
}

/// Paint the primitive's 16x16 signature into the field center — the
/// same instantiation approximation the MERGE/SPLIT ops use.
fn instantiate(engine: &mut CrystalEngine, prim: &Primitive) {
    const S: usize = 16;
    if prim.signature.len() != S * S {
        return;
    }
    let n = engine.field.size;
    let span = n as f64 * 0.35;
    for sy in 0..S {
        for sx in 0..S {
            let v = prim.signature[sy * S + sx];
            if v.abs() < 1e-9 {
                continue;
            }
            let fx = (0.5 * n as f64 + (sx as f64 / S as f64 - 0.5) * span) as isize;
            let fy = (0.5 * n as f64 + (sy as f64 / S as f64 - 0.5) * span) as isize;
            if fx >= 0 && fy >= 0 && (fx as usize) < n && (fy as usize) < n {
                engine.field.u[fy as usize * n + fx as usize] += v * 3.0;
            }
        }
    }
}

fn target_text(seed: u64) -> String {
    format!("behavioral target {seed}")
}

/// One noise-shielding trial: physical recall of a written target after
/// noisy evolution, with vs without the primitive present.
fn noise_shielding_trial(prim: &Primitive, seed: u64) -> f64 {
    let run = |with_prim: bool| {
        let mut engine = trial_engine(prim, seed);
        if with_prim {
            instantiate(&mut engine, prim);
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
/// with vs without the primitive present.
fn pattern_completion_trial(prim: &Primitive, seed: u64) -> f64 {
    let run = |with_prim: bool| {
        let mut engine = trial_engine(prim, seed);
        if with_prim {
            instantiate(&mut engine, prim);
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
    let trial_fn: fn(&Primitive, u64) -> f64 = match capability {
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
        let adv = trial_fn(&prim, seed);
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
    };

    let stored = registry.find_mut(id).ok_or("primitive vanished mid-test")?;
    stored
        .behavioral_capabilities
        .retain(|c| c.name != capability);
    stored.behavioral_capabilities.push(record.clone());
    if passed && stored.evidence_level >= MIN_LEVEL_FOR_PROMOTION {
        stored.evidence_level = stored.evidence_level.max(BEHAVIOR_LEVEL);
    }
    Ok(record)
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
}

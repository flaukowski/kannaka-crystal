//! Versioned algorithm identifiers (ADR-0004 §2). Every experiment
//! manifest stamps these so a result is interpretable after any of the
//! underlying algorithms change. Bump a constant when the corresponding
//! behavior changes in a way that affects results.

/// Crate version — mirrors Cargo.toml.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Leapfrog integrator + Kelvin–Voigt viscosity + graded sponge (ADR-0001).
pub const SOLVER_VERSION: &str = "leapfrog-v1";

/// blake3-seeded band-limited constellation text encoder.
pub const ENCODING_VERSION: &str = "text-wavefront-v1";

/// Observe/prune/amplify/mutate/settle consolidation.
pub const DREAM_VERSION: &str = "dream-v1";

/// Mean+0.5σ threshold, 4-connected flood fill, min-area gate.
pub const DETECTOR_VERSION: &str = "connected-region-v1";

/// Morphological heuristics (area/elongation/annularity/angular gaps).
pub const CLASSIFIER_VERSION: &str = "morphology-v1";

/// 16x16 translation-invariant normalized signature.
pub const SIGNATURE_VERSION: &str = "signature-16x16-v1";

/// persistence*0.7 + retention*0.3 with novelty bonus (discovery).
pub const FITNESS_VERSION: &str = "fitness-v2";

/// Hybrid recall combination (ADR-0004 §3). Weights are versioned —
/// bump the version when retuning, never retune silently.
pub const RECALL_HYBRID_VERSION: &str = "hybrid-v1";
pub const HYBRID_W_PHYSICAL: f64 = 0.45;
pub const HYBRID_W_ENCODING: f64 = 0.15;
pub const HYBRID_W_SEMANTIC: f64 = 0.40;

/// KCB benchmark suite version (ADR-0004 §10).
pub const BENCHMARK_SUITE_VERSION: &str = "KCB-1";

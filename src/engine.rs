//! The Crystal Engine — owns a field, a material, an environment
//! (temperature, noise), the write ledger, and time evolution.

use crate::field::{seeded_rng, Field, DEFAULT_SIZE};
use crate::material::{find_material, Material};
use crate::pulse::{encode_text, write_text, Pulse};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteRecord {
    pub text: String,
    pub importance: f64,
    pub written_at_step: u64,
}

/// Physical-recall probe (ADR-0004 §3): field-state correlations only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub text: String,
    /// 0.6·envelope + 0.4·phase — the physical channel's headline number.
    pub physical_resonance: f64,
    pub envelope_correlation: f64,
    pub phase_correlation: f64,
    pub field_energy: f64,
    pub step: u64,
}

/// One recall hit with every evidence channel preserved (ADR-0004 §3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallResult {
    pub text: String,
    pub physical_resonance: f64,
    pub encoding_similarity: f64,
    pub semantic_similarity: f64,
    pub hybrid_score: f64,
    pub hybrid_version: String,
    pub step: u64,
}

/// Mechanism ablation flags (ADR-0004 §11): individually disable core
/// mechanisms so experiments can determine which one is responsible for
/// an observed effect. Everything defaults ON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ablation {
    pub damping: bool,
    pub nonlinearity: bool,
    pub viscosity: bool,
    pub boundary_reflection: bool,
    pub thermal_noise: bool,
    pub external_noise: bool,
    pub dream_pruning: bool,
    pub dream_amplification: bool,
    pub dream_mutation: bool,
    pub semantic_recall: bool,
}

impl Default for Ablation {
    fn default() -> Self {
        Ablation {
            damping: true,
            nonlinearity: true,
            viscosity: true,
            boundary_reflection: true,
            thermal_noise: true,
            external_noise: true,
            dream_pruning: true,
            dream_amplification: true,
            dream_mutation: true,
            semantic_recall: true,
        }
    }
}

/// Encoder-space cosine between two patterns — encoding recall, fully
/// independent of field evolution.
pub fn pattern_cosine(a: &[f64], b: &[f64]) -> f64 {
    let dot: f64 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let nb: f64 = b.iter().map(|y| y * y).sum::<f64>().sqrt();
    if na < 1e-12 || nb < 1e-12 {
        0.0
    } else {
        (dot / (na * nb)).abs()
    }
}

pub struct CrystalEngine {
    pub field: Field,
    pub material: Material,
    pub temperature_k: f64,
    /// Extra injected noise on top of thermal noise (experiment knob).
    pub noise_amp: f64,
    /// ADR-0004 §11 mechanism switches, all on by default.
    pub ablation: Ablation,
    pub writes: Vec<WriteRecord>,
    /// Energy sampled every `ENERGY_SAMPLE_EVERY` steps for the decay timeline.
    pub energy_timeline: Vec<(u64, f64)>,
    rng: ChaCha8Rng,
}

const ENERGY_SAMPLE_EVERY: u64 = 25;

impl CrystalEngine {
    pub fn new(material_id: &str, size: usize, seed: u64) -> Result<Self, String> {
        let material =
            find_material(material_id).ok_or_else(|| format!("unknown material: {material_id}"))?;
        let temperature_k = material.default_temperature_k;
        Ok(CrystalEngine {
            field: Field::new(size),
            material,
            temperature_k,
            noise_amp: 0.0,
            ablation: Ablation::default(),
            writes: Vec::new(),
            energy_timeline: Vec::new(),
            rng: seeded_rng(seed),
        })
    }

    pub fn default_engine() -> Self {
        Self::new("ideal_resonator", DEFAULT_SIZE, 0).expect("builtin material")
    }

    /// WRITE — inject text as a deterministic wavefront.
    pub fn write(&mut self, text: &str, importance: f64) {
        write_text(&mut self.field, text, importance);
        self.writes.push(WriteRecord {
            text: text.to_string(),
            importance,
            written_at_step: self.field.step_count,
        });
    }

    /// PULSE — inject a parametric pulse.
    pub fn pulse(&mut self, pulse: &Pulse) {
        pulse.apply(&mut self.field);
    }

    /// RESONATE / WAIT — evolve the medium for `steps`.
    pub fn resonate(&mut self, steps: u64) {
        for _ in 0..steps {
            self.field.step(
                &self.material,
                self.temperature_k,
                self.noise_amp,
                &self.ablation,
                &mut self.rng,
            );
            if self.field.step_count.is_multiple_of(ENERGY_SAMPLE_EVERY) {
                self.energy_timeline
                    .push((self.field.step_count, self.field.energy()));
                // Keep the timeline bounded for long-running servers.
                if self.energy_timeline.len() > 4096 {
                    self.energy_timeline.drain(..2048);
                }
            }
        }
    }

    /// PROBE — how strongly does `text` still resonate in the field?
    ///
    /// PHYSICAL recall only (ADR-0004 §3): correlations between the live
    /// medium state and the encoded target — no encoder-side or semantic
    /// similarity leaks in. `physical_resonance` blends envelope
    /// correlation (is the field's energy where this pattern put it?)
    /// with phase correlation (is it still coherently ringing the exact
    /// pattern?); both components are exposed.
    pub fn probe(&mut self, text: &str) -> ProbeResult {
        let pattern = encode_text(text, self.field.size);
        let envelope = self.field.correlate_envelope(&pattern);
        let phase = self.field.correlate(&pattern);
        ProbeResult {
            text: text.to_string(),
            physical_resonance: 0.6 * envelope + 0.4 * phase,
            envelope_correlation: envelope,
            phase_correlation: phase,
            field_energy: self.field.energy(),
            step: self.field.step_count,
        }
    }

    /// RECALL — rank written records against a query through distinct
    /// evidence channels (ADR-0004 §3): physical (field vs candidate
    /// pattern), encoding (encoder-space similarity of query vs candidate,
    /// independent of field evolution), semantic (lexical overlap, outside
    /// the field entirely), and a versioned hybrid used for ranking.
    /// Scientific reports use `physical_resonance` alone; applications may
    /// rank by hybrid, but every component is preserved.
    ///
    /// The `semantic_recall` ablation flag (§11) zeroes the semantic
    /// channel's contribution to the hybrid.
    pub fn recall(&mut self, query: &str, top_k: usize) -> Vec<RecallResult> {
        let query_pattern = encode_text(query, self.field.size);
        let semantic_enabled = self.ablation.semantic_recall;
        let mut results: Vec<RecallResult> = self
            .writes
            .iter()
            .map(|w| w.text.clone())
            .collect::<Vec<_>>()
            .into_iter()
            .map(|text| {
                let probe = self.probe(&text);
                let candidate_pattern = encode_text(&text, self.field.size);
                let encoding_similarity = pattern_cosine(&query_pattern, &candidate_pattern);
                let semantic_similarity = word_overlap(query, &text);
                let semantic_used = if semantic_enabled {
                    semantic_similarity
                } else {
                    0.0
                };
                // hybrid-v1 (versions::RECALL_HYBRID_VERSION): weights are
                // versioned constants, never silently retuned.
                let hybrid_score = crate::versions::HYBRID_W_PHYSICAL * probe.physical_resonance
                    + crate::versions::HYBRID_W_ENCODING * encoding_similarity
                    + crate::versions::HYBRID_W_SEMANTIC * semantic_used;
                RecallResult {
                    text,
                    physical_resonance: probe.physical_resonance,
                    encoding_similarity,
                    semantic_similarity,
                    hybrid_score,
                    hybrid_version: crate::versions::RECALL_HYBRID_VERSION.to_string(),
                    step: probe.step,
                }
            })
            .collect();
        results.sort_by(|a, b| b.hybrid_score.total_cmp(&a.hybrid_score));
        results.truncate(top_k);
        results
    }

    /// Deterministic sub-RNG for consumers that need randomness tied to the
    /// engine's stream (dreaming, discovery).
    pub fn rng(&mut self) -> &mut ChaCha8Rng {
        &mut self.rng
    }
}

fn word_overlap(a: &str, b: &str) -> f64 {
    let wa: std::collections::HashSet<String> = a
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    let wb: std::collections::HashSet<String> = b
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    if wa.is_empty() || wb.is_empty() {
        return 0.0;
    }
    let inter = wa.intersection(&wb).count() as f64;
    inter / wa.len().min(wb.len()) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_then_probe_resonates() {
        let mut e = CrystalEngine::new("ideal_resonator", 64, 42).unwrap();
        e.write("the moon is a resonator", 1.0);
        let fresh = e.probe("the moon is a resonator");
        assert!(
            fresh.physical_resonance > 0.9,
            "fresh write should ring: {}",
            fresh.physical_resonance
        );
        e.resonate(200);
        let later = e.probe("the moon is a resonator");
        let other = e.probe("unrelated content entirely");
        assert!(
            later.physical_resonance > other.physical_resonance,
            "written text ({}) should outresonate unwritten ({})",
            later.physical_resonance,
            other.physical_resonance
        );
    }

    #[test]
    fn recall_ranks_written_memories() {
        let mut e = CrystalEngine::new("optical_cavity", 64, 7).unwrap();
        e.write("echo rings form in cavities", 1.0);
        e.write("silicon disperses quickly", 0.6);
        e.resonate(50);
        let results = e.recall("echo rings", 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].text, "echo rings form in cavities");
    }

    #[test]
    fn decay_reduces_energy_in_lossy_material() {
        let mut e = CrystalEngine::new("silicon", 64, 9).unwrap();
        e.write("fading structure", 1.0);
        e.resonate(10);
        let early = e.field.energy();
        e.resonate(2000);
        let late = e.field.energy();
        assert!(late < early, "silicon should decay: {early} -> {late}");
    }
}

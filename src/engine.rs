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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub text: String,
    /// Correlation of the live field against a fresh encoding of `text`.
    pub resonance: f64,
    pub field_energy: f64,
    pub step: u64,
}

pub struct CrystalEngine {
    pub field: Field,
    pub material: Material,
    pub temperature_k: f64,
    /// Extra injected noise on top of thermal noise (experiment knob).
    pub noise_amp: f64,
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
    /// Blends envelope correlation (is the field's energy where this
    /// pattern put it?) with phase correlation (is it still coherently
    /// ringing the exact pattern?). Envelope dominates: phases evolve
    /// long before a structure dies, and a phase-only probe against an
    /// old field degenerates to chance.
    pub fn probe(&mut self, text: &str) -> ProbeResult {
        let pattern = encode_text(text, self.field.size);
        let envelope = self.field.correlate_envelope(&pattern);
        let phase = self.field.correlate(&pattern);
        ProbeResult {
            text: text.to_string(),
            resonance: 0.6 * envelope + 0.4 * phase,
            field_energy: self.field.energy(),
            step: self.field.step_count,
        }
    }

    /// RECALL — probe every written record and rank by current resonance.
    pub fn recall(&mut self, query: &str, top_k: usize) -> Vec<ProbeResult> {
        // The query itself resonates: correlate the query encoding with each
        // written text's encoding *through the live field* — a probe of the
        // query plus probes of all writes, ranked by combined resonance.
        let query_probe = self.probe(query);
        let mut results: Vec<ProbeResult> = self
            .writes
            .iter()
            .map(|w| w.text.clone())
            .collect::<Vec<_>>()
            .into_iter()
            .map(|text| {
                let mut p = self.probe(&text);
                // Lexical overlap gives a resonance boost — meaning rides on
                // top of the raw wave correlation. The additive term matters:
                // after heavy decay every raw correlation is chance-level
                // (~0.05), and multiplying chance by overlap still loses to
                // chance. Overlap must be able to outweigh noise on its own.
                let overlap = word_overlap(query, &p.text);
                p.resonance = p.resonance * (1.0 + 2.0 * overlap)
                    + 0.3 * overlap
                    + query_probe.resonance * 0.05;
                p
            })
            .collect();
        results.sort_by(|a, b| b.resonance.total_cmp(&a.resonance));
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
            fresh.resonance > 0.9,
            "fresh write should ring: {}",
            fresh.resonance
        );
        e.resonate(200);
        let later = e.probe("the moon is a resonator");
        let other = e.probe("unrelated content entirely");
        assert!(
            later.resonance > other.resonance,
            "written text ({}) should outresonate unwritten ({})",
            later.resonance,
            other.resonance
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

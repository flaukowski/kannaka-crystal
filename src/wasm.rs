//! wasm-bindgen surface for the in-browser engine (Pages demo).
//!
//! The full engine core runs client-side: write, pulse, resonate, dream,
//! probe, detect. Deterministic by construction (ChaCha8-seeded), so a
//! browser session with the same seed replays a lab run exactly.

use crate::dream::{dream, DreamMode};
use crate::engine::CrystalEngine;
use crate::material::builtin_materials;
use crate::primitives::detect_structures;
use crate::pulse::Pulse;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct CrystalWasm {
    engine: CrystalEngine,
}

#[wasm_bindgen]
impl CrystalWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(material: &str, size: usize, seed: u64) -> Result<CrystalWasm, JsValue> {
        let engine = CrystalEngine::new(material, size, seed).map_err(|e| JsValue::from_str(&e))?;
        Ok(CrystalWasm { engine })
    }

    pub fn write(&mut self, text: &str, importance: f64) {
        self.engine.write(text, importance);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn pulse(&mut self, x: f64, y: f64, radius: f64, amplitude: f64, frequency: f64, phase: f64) {
        self.engine.pulse(&Pulse { x, y, radius, amplitude, frequency, phase });
    }

    pub fn resonate(&mut self, steps: u32) {
        self.engine.resonate(steps as u64);
    }

    /// Resonance of `text` against the live field (0..~1).
    pub fn probe(&mut self, text: &str) -> f64 {
        self.engine.probe(text).resonance
    }

    /// Run a dream cycle; returns a small JSON report.
    pub fn dream(&mut self, deep: bool) -> String {
        let mode = if deep { DreamMode::Deep } else { DreamMode::Light };
        let r = dream(&mut self.engine, mode);
        format!(
            "{{\"pruned_fraction\":{:.4},\"energy_before\":{:.3},\"energy_after\":{:.3}}}",
            r.pruned_fraction, r.energy_before, r.energy_after
        )
    }

    /// Detect stable structures in the current field; returns JSON summary.
    pub fn detect(&mut self) -> String {
        let r = dream(&mut self.engine, DreamMode::Light);
        let found = detect_structures(&r.stability_map, self.engine.field.size);
        let items: Vec<String> = found
            .iter()
            .take(8)
            .map(|s| {
                format!(
                    "{{\"class\":\"{}\",\"stability\":{:.2},\"area\":{},\"x\":{:.3},\"y\":{:.3}}}",
                    s.class, s.stability_score, s.area, s.centroid.0, s.centroid.1
                )
            })
            .collect();
        format!("[{}]", items.join(","))
    }

    pub fn set_noise(&mut self, amp: f64) {
        self.engine.noise_amp = amp.max(0.0);
    }

    pub fn set_temperature(&mut self, kelvin: f64) {
        self.engine.temperature_k = kelvin.max(0.0);
    }

    /// Downsampled |u| for rendering.
    pub fn field(&self, out_size: usize) -> Vec<f64> {
        self.engine.field.downsample_abs(out_size)
    }

    pub fn energy(&self) -> f64 {
        self.engine.field.energy()
    }

    pub fn step_count(&self) -> f64 {
        self.engine.field.step_count as f64
    }

    pub fn material_id(&self) -> String {
        self.engine.material.id.clone()
    }
}

/// Material library as JSON (id, name, description).
#[wasm_bindgen]
pub fn materials_json() -> String {
    let mats = builtin_materials();
    let items: Vec<String> = mats
        .iter()
        .map(|m| {
            format!(
                "{{\"id\":\"{}\",\"name\":\"{}\"}}",
                m.id,
                m.name.replace('"', "'")
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

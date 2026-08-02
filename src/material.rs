//! Material plugins — each material is a parameter set that shapes how the
//! resonant medium propagates, damps, reflects, and saturates waves.
//!
//! Everything here is a *model*, not a physics claim (PRD: H1–H3 are
//! investigated empirically; no new physics is assumed). Built-in materials
//! are calibrated so their qualitative behavior differs enough to matter:
//! a vacuum barely rings, an optical cavity rings nearly forever, europium
//! crystal holds structure at cryogenic temperature but decays fast warm.

use serde::{Deserialize, Serialize};

/// Material-model category (ADR-0004 §1). Every built-in preset is
/// phenomenological: a parameter set that exhibits a useful qualitative
/// behavior, with no claim to the governing physics of the named
/// material. Physical / calibrated / hardware-profile models are earned
/// through evidence, never assumed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModelKind {
    #[default]
    Phenomenological,
    Physical,
    Calibrated,
    HardwareProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    /// Stable identifier, e.g. `optical_cavity`. NEVER renamed — swarm
    /// unit files, KannakaHDL queries, and registry rows reference it.
    pub id: String,
    pub name: String,
    pub description: String,
    /// ADR-0004 §1 model classification. Serde defaults keep old
    /// user-defined material JSON loadable.
    #[serde(default)]
    pub model_kind: ModelKind,
    /// "uncalibrated" until a calibration dataset is attached.
    #[serde(default = "default_validation_status")]
    pub validation_status: String,
    /// What real system inspired the parameters (empty = abstract).
    #[serde(default)]
    pub inspired_by: Vec<String>,
    /// Literature references backing a physical/calibrated claim.
    #[serde(default)]
    pub references: Vec<String>,
    /// Wave propagation speed in grid units per step (CFL-bounded by the engine).
    pub wave_speed: f64,
    /// Bulk amplitude damping per step (decay is information, not loss).
    pub damping: f64,
    /// Boundary reflection coefficient in [0, 1]; 1.0 = perfect mirror.
    pub boundary_reflect: f64,
    /// Cubic saturation strength — softly limits amplitude, which is what
    /// lets standing structures stabilize instead of blowing up.
    pub nonlinearity: f64,
    /// Default operating temperature in Kelvin.
    pub default_temperature_k: f64,
    /// How strongly temperature converts into per-step thermal noise.
    pub thermal_noise_coupling: f64,
}

fn default_validation_status() -> String {
    "uncalibrated".into()
}

/// Metadata defaults shared by every built-in preset (ADR-0004 §1):
/// phenomenological, uncalibrated, no physical claim. Physics fields are
/// placeholders — every builtin overrides them via struct update.
fn phenomenological_defaults() -> Material {
    Material {
        id: String::new(),
        name: String::new(),
        description: String::new(),
        model_kind: ModelKind::Phenomenological,
        validation_status: default_validation_status(),
        inspired_by: Vec::new(),
        references: Vec::new(),
        wave_speed: 0.0,
        damping: 0.0,
        boundary_reflect: 0.0,
        nonlinearity: 0.0,
        default_temperature_k: 0.0,
        thermal_noise_coupling: 0.0,
    }
}

impl Material {
    /// Effective per-step noise amplitude at a given temperature.
    pub fn thermal_noise(&self, temperature_k: f64) -> f64 {
        self.thermal_noise_coupling * (temperature_k / 300.0).max(0.0)
    }
}

/// Built-in material plugins (PRD "Material Plugins"). User-defined materials
/// can be loaded from JSON via [`Material`]'s `Deserialize` impl.
///
/// Display names for real-substance-inspired presets say "-Inspired"
/// (ADR-0004 §1): evocative naming stays, implied physical fidelity goes.
pub fn builtin_materials() -> Vec<Material> {
    vec![
        Material {
            id: "vacuum".into(),
            name: "Vacuum".into(),
            description: "Idealized empty medium — fast propagation, no damping, \
                          absorbing boundaries. Structures disperse quickly."
                .into(),
            // 0.6, not the 2D CFL limit (1/sqrt(2) ~= 0.707): running at the
            // marginal speed makes the leapfrog scheme itself pump energy,
            // which masquerades as retention in an absorbing medium.
            wave_speed: 0.6,
            damping: 0.0,
            boundary_reflect: 0.0,
            nonlinearity: 0.0,
            default_temperature_k: 2.7,
            thermal_noise_coupling: 0.0,
            ..phenomenological_defaults()
        },
        Material {
            id: "ideal_resonator".into(),
            name: "Ideal Resonator".into(),
            description: "Lossless mirrored cavity. The reference medium for \
                          discovering primitives without decay pressure."
                .into(),
            wave_speed: 0.5,
            damping: 0.0002,
            boundary_reflect: 1.0,
            nonlinearity: 0.02,
            default_temperature_k: 0.0,
            thermal_noise_coupling: 0.0,
            ..phenomenological_defaults()
        },
        Material {
            id: "optical_cavity".into(),
            name: "Optical-Cavity-Inspired Medium".into(),
            inspired_by: vec!["high-Q optical cavities".into()],
            description: "High-Q cavity: slow leak through partially silvered \
                          boundaries, mild saturation. Long-lived echo rings."
                .into(),
            wave_speed: 0.6,
            damping: 0.001,
            boundary_reflect: 0.97,
            nonlinearity: 0.015,
            default_temperature_k: 293.0,
            thermal_noise_coupling: 0.0005,
            ..phenomenological_defaults()
        },
        Material {
            id: "europium_crystal".into(),
            name: "Europium-Inspired Resonant Medium".into(),
            inspired_by: vec!["Cs2NaEuF6 rare-earth optical memory systems".into()],
            description: "Rare-earth doped crystal model. Excellent persistence \
                          when cold (4 K), rapid dephasing when warm."
                .into(),
            wave_speed: 0.45,
            damping: 0.0008,
            boundary_reflect: 0.92,
            nonlinearity: 0.03,
            default_temperature_k: 4.0,
            thermal_noise_coupling: 0.004,
            ..phenomenological_defaults()
        },
        Material {
            id: "silicon".into(),
            name: "Silicon-Inspired Medium".into(),
            inspired_by: vec!["crystalline silicon acoustics".into()],
            description: "Workhorse solid-state medium: moderate damping, \
                          moderate reflection, tolerant of room temperature."
                .into(),
            wave_speed: 0.55,
            damping: 0.003,
            boundary_reflect: 0.85,
            nonlinearity: 0.01,
            default_temperature_k: 293.0,
            thermal_noise_coupling: 0.001,
            ..phenomenological_defaults()
        },
        Material {
            id: "diamond_nv".into(),
            name: "Diamond-NV-Inspired Medium".into(),
            inspired_by: vec!["diamond nitrogen-vacancy centers".into()],
            description: "Nitrogen-vacancy model: stiff lattice (fast waves), \
                          low damping, strong local nonlinearity around centers."
                .into(),
            wave_speed: 0.65,
            damping: 0.0005,
            boundary_reflect: 0.9,
            nonlinearity: 0.05,
            default_temperature_k: 293.0,
            thermal_noise_coupling: 0.0008,
            ..phenomenological_defaults()
        },
        Material {
            id: "metamaterial".into(),
            name: "Artificial Meta-material".into(),
            description: "Engineered dispersion: slow waves, near-perfect \
                          mirrors, strong saturation. Built to trap structure."
                .into(),
            wave_speed: 0.35,
            damping: 0.0004,
            boundary_reflect: 0.99,
            nonlinearity: 0.08,
            default_temperature_k: 293.0,
            thermal_noise_coupling: 0.0003,
            ..phenomenological_defaults()
        },
        Material {
            id: "graphene_model".into(),
            name: "Graphene-Inspired Model".into(),
            inspired_by: vec!["graphene membrane dynamics".into()],
            description: "2D sheet model: very fast propagation, weak damping, \
                          leaky edges. Structures spread wide and thin."
                .into(),
            wave_speed: 0.69,
            damping: 0.0009,
            boundary_reflect: 0.6,
            nonlinearity: 0.005,
            default_temperature_k: 293.0,
            thermal_noise_coupling: 0.0012,
            ..phenomenological_defaults()
        },
    ]
}

pub fn find_material(id: &str) -> Option<Material> {
    builtin_materials().into_iter().find(|m| m.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_have_unique_ids_and_sane_ranges() {
        let mats = builtin_materials();
        let mut ids: Vec<_> = mats.iter().map(|m| m.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), mats.len(), "duplicate material ids");
        for m in &mats {
            assert!(m.wave_speed > 0.0 && m.wave_speed <= 0.7, "{} CFL", m.id);
            assert!((0.0..=1.0).contains(&m.boundary_reflect), "{}", m.id);
            assert!(m.damping >= 0.0 && m.damping < 0.1, "{}", m.id);
            // ADR-0004 §1: every builtin is an honestly-labeled
            // phenomenological, uncalibrated model; anything named after a
            // real substance says so in its display name.
            assert_eq!(m.model_kind, ModelKind::Phenomenological, "{}", m.id);
            assert_eq!(m.validation_status, "uncalibrated", "{}", m.id);
            if !m.inspired_by.is_empty() {
                assert!(m.name.contains("Inspired"), "{}: {}", m.id, m.name);
            }
        }
    }

    #[test]
    fn europium_noise_scales_with_temperature() {
        let eu = find_material("europium_crystal").unwrap();
        assert!(eu.thermal_noise(300.0) > 50.0 * eu.thermal_noise(4.0));
    }
}

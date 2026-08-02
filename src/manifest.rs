//! Experiment manifests (ADR-0004 §2): every experiment produces an
//! immutable, machine-readable record sufficient to reproduce it — and
//! every registered primitive references the manifest that produced it.
//!
//! Manifests are written to `<data_dir>/experiments/<experiment_id>.json`.
//! The **experiment hash** is derived from the generating protocol only
//! (model, solver, environment, seed, versioned algorithms, program) —
//! never from results or timestamps — so two runs of the same protocol
//! hash identically and reproductions are detectable (ADR-0004 §6).

use crate::material::{Material, ModelKind};
use crate::versions::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialRef {
    pub id: String,
    pub model_kind: ModelKind,
    pub validation_status: String,
    /// The actual physics parameters in force, so a preset change after
    /// the fact cannot silently reinterpret an old experiment.
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub temperature_k: f64,
    pub noise_amplitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentManifest {
    pub schema_version: String,
    pub experiment_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub engine_version: String,
    /// Populated when the build embeds it (CI exports KC_GIT_COMMIT).
    pub git_commit: Option<String>,
    pub solver_version: String,
    pub field_size: usize,
    pub seed: u64,
    pub material: MaterialRef,
    pub environment: Environment,
    pub encoding_version: String,
    pub dream_version: String,
    pub detector_version: String,
    pub classifier_version: String,
    pub signature_version: String,
    pub fitness_version: String,
    /// What was run: `{"kind":"crystal-program","source":...}` or
    /// `{"kind":"evolve","config":{...}}`.
    pub program: serde_json::Value,
    /// Filled in at completion; excluded from the experiment hash.
    pub results: serde_json::Value,
    pub artifacts: Vec<String>,
    pub parent_experiments: Vec<Uuid>,
}

impl ExperimentManifest {
    pub fn new(
        material: &Material,
        field_size: usize,
        seed: u64,
        temperature_k: f64,
        noise_amplitude: f64,
        program: serde_json::Value,
    ) -> Self {
        ExperimentManifest {
            schema_version: "1".into(),
            experiment_id: Uuid::new_v4(),
            started_at: Utc::now(),
            engine_version: ENGINE_VERSION.into(),
            git_commit: option_env!("KC_GIT_COMMIT").map(|s| s.to_string()),
            solver_version: SOLVER_VERSION.into(),
            field_size,
            seed,
            material: MaterialRef {
                id: material.id.clone(),
                model_kind: material.model_kind,
                validation_status: material.validation_status.clone(),
                parameters: serde_json::json!({
                    "wave_speed": material.wave_speed,
                    "damping": material.damping,
                    "boundary_reflect": material.boundary_reflect,
                    "nonlinearity": material.nonlinearity,
                    "thermal_noise_coupling": material.thermal_noise_coupling,
                }),
            },
            environment: Environment {
                temperature_k,
                noise_amplitude,
            },
            encoding_version: ENCODING_VERSION.into(),
            dream_version: DREAM_VERSION.into(),
            detector_version: DETECTOR_VERSION.into(),
            classifier_version: CLASSIFIER_VERSION.into(),
            signature_version: SIGNATURE_VERSION.into(),
            fitness_version: FITNESS_VERSION.into(),
            program,
            results: serde_json::Value::Null,
            artifacts: Vec::new(),
            parent_experiments: Vec::new(),
        }
    }

    /// Deterministic hash of the generating protocol (ADR-0004 §6).
    /// Excludes experiment_id, started_at, results, and artifacts —
    /// same protocol, same hash, on any machine.
    pub fn experiment_hash(&self) -> String {
        let protocol = serde_json::json!({
            "schema_version": self.schema_version,
            "engine_version": self.engine_version,
            "solver_version": self.solver_version,
            "field_size": self.field_size,
            "seed": self.seed,
            "material": self.material,
            "environment": self.environment,
            "encoding_version": self.encoding_version,
            "dream_version": self.dream_version,
            "detector_version": self.detector_version,
            "classifier_version": self.classifier_version,
            "signature_version": self.signature_version,
            "fitness_version": self.fitness_version,
            "program": self.program,
            "parent_experiments": self.parent_experiments,
        });
        blake3::hash(protocol.to_string().as_bytes())
            .to_hex()
            .to_string()
    }

    /// Persist to `<data_dir>/experiments/<id>.json` (write-then-rename).
    pub fn save(&self) -> Result<std::path::PathBuf, String> {
        self.save_to(&crate::registry::data_dir())
    }

    pub fn save_to(&self, data_dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
        let dir = data_dir.join("experiments");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!("{}.json", self.experiment_id));
        let tmp = dir.join(format!("{}.json.tmp", self.experiment_id));
        std::fs::write(
            &tmp,
            serde_json::to_string_pretty(self).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::find_material;

    fn manifest(seed: u64) -> ExperimentManifest {
        ExperimentManifest::new(
            &find_material("metamaterial").unwrap(),
            96,
            seed,
            293.0,
            0.0,
            serde_json::json!({"kind": "evolve", "generations": 3}),
        )
    }

    #[test]
    fn experiment_hash_is_protocol_deterministic() {
        let a = manifest(42);
        let mut b = manifest(42);
        // Different run identity + results, same protocol -> same hash.
        b.results = serde_json::json!({"discovered": 7});
        assert_ne!(a.experiment_id, b.experiment_id);
        assert_eq!(a.experiment_hash(), b.experiment_hash());
        // Any protocol change -> different hash.
        assert_ne!(
            manifest(42).experiment_hash(),
            manifest(43).experiment_hash()
        );
    }

    #[test]
    fn manifest_roundtrips_and_saves() {
        // Explicit dir, not the env var — parallel tests share the process
        // environment and an env-var race here would be flaky.
        let dir = std::env::temp_dir().join(format!("kc-man-{}", std::process::id()));
        let m = manifest(1);
        let path = m.save_to(&dir).unwrap();
        let loaded: ExperimentManifest =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(loaded.experiment_id, m.experiment_id);
        assert_eq!(loaded.experiment_hash(), m.experiment_hash());
        assert_eq!(
            loaded.material.model_kind,
            crate::material::ModelKind::Phenomenological
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}

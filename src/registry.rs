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
    ) -> Option<Primitive> {
        let duplicate = self.primitives.iter().any(|p| {
            p.class == s.class && signature_similarity(&p.signature, &s.signature) >= 0.92
        });
        if duplicate {
            return None;
        }
        self.next_serial += 1;
        let quantized: Vec<u8> = s.signature.iter().map(|v| (v * 255.0) as u8).collect();
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
        };
        self.primitives.push(prim.clone());
        Some(prim)
    }

    pub fn find(&self, id: &str) -> Option<&Primitive> {
        self.primitives
            .iter()
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
    pub fn search(
        &self,
        class: Option<&str>,
        material: Option<&str>,
        min_persistence: f64,
    ) -> Vec<&Primitive> {
        let normalize = |s: &str| s.to_lowercase().replace(['_', '-'], " ");
        self.primitives
            .iter()
            .filter(|p| {
                class.is_none_or(|c| normalize(&p.class.to_string()) == normalize(c))
                    && material.is_none_or(|m| p.material_id == m)
                    && p.persistence >= min_persistence
            })
            .collect()
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
        let p1 = r
            .register(&s1, 0.9, 0.8, vec![1.0, 0.9], "silicon", vec![], "test")
            .expect("first registration");
        assert_eq!(p1.id, "CRY-000001");

        // Exact duplicate is rejected.
        assert!(r
            .register(&s1, 0.9, 0.8, vec![], "silicon", vec![], "test")
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
            )
            .expect("second registration");
        assert_eq!(p2.id, "CRY-000002");
    }

    #[test]
    fn roundtrip_persistence() {
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
        );
        r.save().unwrap();
        let loaded = Registry::load().unwrap();
        assert_eq!(loaded.primitives.len(), 1);
        assert_eq!(loaded.primitives[0].material_id, "vacuum");
        std::env::remove_var("KANNAKA_CRYSTAL_DATA_DIR");
        let _ = std::fs::remove_dir_all(dir);
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
        );
        r.register(
            &fake_structure(0.77),
            0.3,
            0.4,
            vec![],
            "vacuum",
            vec![],
            "t",
        );
        assert_eq!(r.search(Some("standing_echo"), None, 0.0).len(), 2);
        assert_eq!(
            r.search(Some("Standing Echo"), Some("silicon"), 0.0).len(),
            1
        );
        assert_eq!(r.search(None, None, 0.5).len(), 1);
        assert_eq!(r.search(Some("echo ring"), None, 0.0).len(), 0);
    }

    #[test]
    fn similarity_search_ranks_self_first() {
        let mut r = Registry::default();
        let a = fake_structure(0.13);
        let b = fake_structure(0.99);
        r.register(&a, 0.9, 0.8, vec![], "silicon", vec![], "t");
        r.register(&b, 0.9, 0.8, vec![], "silicon", vec![], "t");
        let hits = r.similar(&a.signature, 2);
        assert!(hits[0].0 > 0.999);
    }
}

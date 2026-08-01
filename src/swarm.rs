//! NATS swarm integration (feature = "swarm").
//!
//! Specialist agents (Explorer, Optimizer, Classifier, Dreamer, Physicist,
//! Archivist) coordinate over NATS subjects:
//!
//!   kannaka.crystal.events                 — all lifecycle events (JSON)
//!   kannaka.crystal.primitive.discovered   — new registry entries
//!   kannaka.crystal.explore.request        — work requests for explorers
//!
//! Connection comes from `KANNAKA_NATS_URL` (default `nats://127.0.0.1:4222`)
//! and optional `KANNAKA_NATS_CREDS` (path to a .creds file). Credentials are
//! never read from the repo or hardcoded.

use crate::discovery::{evolve, EvolutionConfig};
use crate::registry::{Primitive, Registry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrystalEvent {
    pub kind: String,
    pub node: String,
    pub at: chrono::DateTime<chrono::Utc>,
    pub payload: serde_json::Value,
}

pub const SUBJECT_EVENTS: &str = "kannaka.crystal.events";
pub const SUBJECT_DISCOVERED: &str = "kannaka.crystal.primitive.discovered";
pub const SUBJECT_EXPLORE: &str = "kannaka.crystal.explore.request";

fn nats_url() -> String {
    std::env::var("KANNAKA_NATS_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".into())
}

fn node_name() -> String {
    std::env::var("KANNAKA_CRYSTAL_NODE")
        .unwrap_or_else(|_| format!("crystal-{}", &uuid::Uuid::new_v4().to_string()[..8]))
}

pub fn connect() -> Result<nats::Connection, String> {
    let url = nats_url();
    let options = match std::env::var("KANNAKA_NATS_CREDS") {
        Ok(creds) => nats::Options::with_credentials(creds),
        Err(_) => nats::Options::new(),
    };
    options
        .with_name("kannaka-crystal")
        .connect(&url)
        .map_err(|e| format!("NATS connect {url}: {e}"))
}

pub fn publish_event(
    conn: &nats::Connection,
    kind: &str,
    payload: serde_json::Value,
) -> Result<(), String> {
    let event = CrystalEvent {
        kind: kind.to_string(),
        node: node_name(),
        at: chrono::Utc::now(),
        payload,
    };
    let bytes = serde_json::to_vec(&event).map_err(|e| e.to_string())?;
    conn.publish(SUBJECT_EVENTS, &bytes)
        .map_err(|e| e.to_string())?;
    if kind == "primitive.discovered" {
        conn.publish(SUBJECT_DISCOVERED, &bytes)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn announce_primitive(conn: &nats::Connection, prim: &Primitive) -> Result<(), String> {
    publish_event(
        conn,
        "primitive.discovered",
        serde_json::to_value(prim).unwrap(),
    )
}

/// Run an Explorer agent: subscribe to explore requests, run bounded
/// evolutionary searches, announce discoveries. Blocks forever.
pub fn run_explorer(material_id: &str) -> Result<(), String> {
    let conn = connect()?;
    let node = node_name();
    println!("explorer {node} online — material {material_id}, subject {SUBJECT_EXPLORE}");
    publish_event(
        &conn,
        "explorer.online",
        serde_json::json!({ "material": material_id }),
    )?;

    let sub = conn.subscribe(SUBJECT_EXPLORE).map_err(|e| e.to_string())?;
    // Also self-schedule: explore continuously with rotating seeds even when
    // nobody is publishing requests.
    let mut seed: u64 = rand::random();
    loop {
        // Drain any explicit request first (non-blocking).
        if let Some(msg) = sub.try_next() {
            if let Ok(req) = serde_json::from_slice::<EvolutionConfig>(&msg.data) {
                explore_once(&conn, &req)?;
                continue;
            }
        }
        seed = seed.wrapping_add(1);
        let cfg = EvolutionConfig {
            material_id: material_id.to_string(),
            generations: 3,
            population: 8,
            seed,
            ..Default::default()
        };
        explore_once(&conn, &cfg)?;
    }
}

fn explore_once(conn: &nats::Connection, cfg: &EvolutionConfig) -> Result<(), String> {
    let mut registry = Registry::load().map_err(|e| e.to_string())?;
    let before = registry.primitives.len();
    let report = evolve(cfg, &mut registry, |line| println!("{line}"));
    registry.save().map_err(|e| e.to_string())?;
    for prim in &registry.primitives[before..] {
        announce_primitive(conn, prim)?;
    }
    publish_event(
        conn,
        "explore.completed",
        serde_json::json!({
            "material": cfg.material_id,
            "seed": cfg.seed,
            "evaluated": report.evaluated,
            "discovered": report.discovered.len(),
        }),
    )
}

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
//!
//! Built on async-nats — the deprecated sync `nats` crate pins a TLS stack
//! (ring 0.16, rustls-webpki 0.100/0.101) with open RUSTSEC advisories.

use crate::discovery::{evolve, EvolutionConfig};
use crate::registry::{Primitive, Registry};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::time::Duration;

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

async fn connect() -> Result<async_nats::Client, String> {
    let url = nats_url();
    let mut options = async_nats::ConnectOptions::new().name("kannaka-crystal");
    if let Ok(creds) = std::env::var("KANNAKA_NATS_CREDS") {
        options = options
            .credentials_file(&creds)
            .await
            .map_err(|e| format!("NATS creds {creds}: {e}"))?;
    }
    options
        .connect(&url)
        .await
        .map_err(|e| format!("NATS connect {url}: {e}"))
}

async fn publish_event(
    client: &async_nats::Client,
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
    client
        .publish(SUBJECT_EVENTS, bytes.clone().into())
        .await
        .map_err(|e| e.to_string())?;
    if kind == "primitive.discovered" {
        client
            .publish(SUBJECT_DISCOVERED, bytes.into())
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn announce_primitive(client: &async_nats::Client, prim: &Primitive) -> Result<(), String> {
    publish_event(
        client,
        "primitive.discovered",
        serde_json::to_value(prim).unwrap(),
    )
    .await
}

/// Run an Explorer agent: subscribe to explore requests, run bounded
/// evolutionary searches, announce discoveries. Blocks forever.
pub fn run_explorer(material_id: &str) -> Result<(), String> {
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    runtime.block_on(run_explorer_async(material_id))
}

async fn run_explorer_async(material_id: &str) -> Result<(), String> {
    let client = connect().await?;
    let node = node_name();
    println!("explorer {node} online — material {material_id}, subject {SUBJECT_EXPLORE}");
    publish_event(
        &client,
        "explorer.online",
        serde_json::json!({ "material": material_id }),
    )
    .await?;

    let mut sub = client
        .subscribe(SUBJECT_EXPLORE)
        .await
        .map_err(|e| e.to_string())?;

    // Also self-schedule: explore continuously with rotating seeds even when
    // nobody is publishing requests.
    let mut seed: u64 = rand::random();
    loop {
        // Drain any explicit request first (bounded wait so the self-driven
        // loop keeps moving when the subject is quiet).
        let requested = match tokio::time::timeout(Duration::from_millis(250), sub.next()).await {
            Ok(Some(msg)) => serde_json::from_slice::<EvolutionConfig>(&msg.payload).ok(),
            _ => None,
        };
        let cfg = requested.unwrap_or_else(|| {
            seed = seed.wrapping_add(1);
            EvolutionConfig {
                material_id: material_id.to_string(),
                generations: 3,
                population: 8,
                seed,
                ..Default::default()
            }
        });
        explore_once(&client, &cfg).await?;
    }
}

async fn explore_once(client: &async_nats::Client, cfg: &EvolutionConfig) -> Result<(), String> {
    let mut registry = Registry::load().map_err(|e| e.to_string())?;
    let before = registry.primitives.len();
    // CPU-bound search; this explorer does one thing, so blocking the
    // runtime between NATS interactions is fine.
    let report = evolve(cfg, &mut registry, |line| println!("{line}"));
    registry.save().map_err(|e| e.to_string())?;
    for prim in &registry.primitives[before..] {
        announce_primitive(client, prim).await?;
    }
    publish_event(
        client,
        "explore.completed",
        serde_json::json!({
            "material": cfg.material_id,
            "seed": cfg.seed,
            "evaluated": report.evaluated,
            "discovered": report.discovered.len(),
        }),
    )
    .await
}

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
    } else if let (Ok(user), Ok(pass)) =
        (std::env::var("NATS_USER"), std::env::var("NATS_PASSWORD"))
    {
        // Constellation convention (kannaka-memory, radio, disk-monitor all
        // read these): the swarm server's ANONYMOUS user is a read-only
        // mirror limited to a curated subject allowlist, and it denies
        // everything else SILENTLY — publishes vanish and subscriptions
        // yield nothing. kannaka.crystal.* needs the authenticated user.
        options = options.user_and_password(user, pass);
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

/// Drain pending discovery announcements from other nodes and merge them
/// into the local registry, so this node's novelty search is swarm-wide
/// (PRD v0.5: thousands of agents must not rediscover each other's work).
async fn merge_inbound(
    sub: &mut async_nats::Subscriber,
    registry: &mut Registry,
    self_node: &str,
) -> usize {
    let mut merged = 0usize;
    while let Ok(Some(msg)) = tokio::time::timeout(Duration::from_millis(50), sub.next()).await {
        let Ok(event) = serde_json::from_slice::<CrystalEvent>(&msg.payload) else {
            continue;
        };
        if event.node == self_node {
            continue;
        }
        let Ok(prim) = serde_json::from_value::<Primitive>(event.payload) else {
            continue;
        };
        if let Some(local_id) = registry.merge_remote(&prim, &event.node) {
            println!("  merged {} from {} as {local_id}", prim.id, event.node);
            merged += 1;
        }
    }
    merged
}

/// Run an Explorer agent: merge swarm discoveries, serve explicit explore
/// requests, otherwise self-schedule searches rotating through `materials`
/// with fresh seeds. Blocks forever.
pub fn run_explorer(materials: &[String]) -> Result<(), String> {
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    runtime.block_on(run_explorer_async(materials))
}

async fn run_explorer_async(materials: &[String]) -> Result<(), String> {
    let client = connect().await?;
    let node = node_name();
    println!("explorer {node} online — materials {materials:?}, subject {SUBJECT_EXPLORE}");
    publish_event(
        &client,
        "explorer.online",
        serde_json::json!({ "materials": materials }),
    )
    .await?;

    let mut requests = client
        .subscribe(SUBJECT_EXPLORE)
        .await
        .map_err(|e| e.to_string())?;
    let mut discoveries = client
        .subscribe(SUBJECT_DISCOVERED)
        .await
        .map_err(|e| e.to_string())?;

    let mut seed: u64 = rand::random();
    let mut round = 0usize;
    loop {
        // Swarm sync first: what others found is not novel here anymore.
        let mut registry = Registry::load().map_err(|e| e.to_string())?;
        if merge_inbound(&mut discoveries, &mut registry, &node).await > 0 {
            registry.save().map_err(|e| e.to_string())?;
        }

        // Explicit request beats self-scheduling (bounded wait keeps the
        // self-driven loop moving when the subject is quiet).
        let requested =
            match tokio::time::timeout(Duration::from_millis(250), requests.next()).await {
                Ok(Some(msg)) => serde_json::from_slice::<EvolutionConfig>(&msg.payload).ok(),
                _ => None,
            };
        let cfg = requested.unwrap_or_else(|| {
            seed = seed.wrapping_add(1);
            round += 1;
            EvolutionConfig {
                material_id: materials[round % materials.len()].clone(),
                generations: 3,
                population: 8,
                seed,
                ..Default::default()
            }
        });
        explore_once(&client, &cfg).await?;
    }
}

/// Run an Archivist agent: merge every discovery announced on the swarm
/// into this node's registry. Point it at the data dir the Observatory
/// serves and the registry grows live as explorers work. Blocks forever.
pub fn run_archivist() -> Result<(), String> {
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    runtime.block_on(async {
        let client = connect().await?;
        let node = node_name();
        println!("archivist {node} online — merging {SUBJECT_DISCOVERED}");
        publish_event(&client, "archivist.online", serde_json::json!({})).await?;

        let mut discoveries = client
            .subscribe(SUBJECT_DISCOVERED)
            .await
            .map_err(|e| e.to_string())?;
        loop {
            let Some(msg) = discoveries.next().await else {
                return Err("discovery subscription closed".to_string());
            };
            let Ok(event) = serde_json::from_slice::<CrystalEvent>(&msg.payload) else {
                continue;
            };
            let Ok(prim) = serde_json::from_value::<Primitive>(event.payload.clone()) else {
                continue;
            };
            // Load-merge-save per event: the registry file is shared with
            // the Observatory server, which also reloads per request.
            let mut registry = Registry::load().map_err(|e| e.to_string())?;
            if let Some(local_id) = registry.merge_remote(&prim, &event.node) {
                registry.save().map_err(|e| e.to_string())?;
                println!(
                    "archived {local_id} <- {}@{} ({}, persistence {:.1}%)",
                    prim.id,
                    event.node,
                    prim.class,
                    prim.persistence * 100.0
                );
            }
        }
    })
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

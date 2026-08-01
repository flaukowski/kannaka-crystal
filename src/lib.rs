//! Kannaka Crystal — an experimental platform for informational materials
//! and resonant memory systems.
//!
//! Memory here is not a storage location: it is an emergent property of
//! interference, resonance, decay, consolidation, and recall in a simulated
//! resonant medium. The crate provides:
//!
//! - [`field`] / [`engine`] — the Crystal Engine: wave propagation,
//!   signal injection, material modeling, noise, temperature, energy.
//! - [`material`] — material plugins (vacuum … europium crystal … metamaterial).
//! - [`pulse`] — signal injection and deterministic text→wavefront encoding.
//! - [`dream`] — offline consolidation (compress / mutate / prune / rank).
//! - [`primitives`] — detection + classification of stable informational
//!   geometries (Echo Rings, Standing Echoes, Phase Knots, Memory Seeds…).
//! - [`registry`] — the Crystal Registry: persistent identity + lineage.
//! - [`discovery`] — evolutionary search for novel primitives.
//! - [`lang`] — the Crystal Language (`.crystal` programs).
//! - [`api`] — REST API + embedded Observatory.
//! - `swarm` (feature `swarm`) — NATS agents (Explorer et al.).

pub mod api;
pub mod discovery;
pub mod dream;
pub mod engine;
pub mod field;
pub mod lang;
pub mod material;
pub mod primitives;
#[cfg(feature = "publish")]
pub mod publish;
pub mod pulse;
pub mod registry;
#[cfg(feature = "swarm")]
pub mod swarm;

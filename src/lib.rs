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

// The engine core (field/material/pulse/engine/dream/primitives) is
// target-agnostic and compiles to wasm32 for the in-browser Pages demo.
// Everything touching the filesystem, network, or clocks stays native.
pub mod dream;
pub mod engine;
pub mod field;
pub mod material;
pub mod primitives;
pub mod pulse;

#[cfg(not(target_arch = "wasm32"))]
pub mod api;
#[cfg(not(target_arch = "wasm32"))]
pub mod discovery;
#[cfg(not(target_arch = "wasm32"))]
pub mod lang;
#[cfg(all(not(target_arch = "wasm32"), feature = "publish"))]
pub mod publish;
#[cfg(not(target_arch = "wasm32"))]
pub mod registry;
#[cfg(all(not(target_arch = "wasm32"), feature = "swarm"))]
pub mod swarm;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

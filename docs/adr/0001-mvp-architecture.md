# ADR-0001: MVP 0.1 Architecture — Simulation Only

Status: Accepted · Date: 2026-08-01

## Context

The Kannaka Crystal PRD calls for a platform to discover "informational
materials" — media whose computational behavior emerges from resonance
rather than explicit programming. Version 0.1 is explicitly simulation-only
(no quantum or laboratory hardware) and must ship: resonance simulator,
pulse generator, wave engine, observatory, primitive registry, dream
engine, NATS integration, and a REST API.

## Decision

**Single Rust crate, one binary (`kannaka-crystal`), lean dependencies.**

- **Medium**: a 2D scalar field on an N×N grid, leapfrog integration of the
  damped, softly-saturating wave equation. 2D is the smallest space where
  rings, knots, and bridges exist as geometries; 3D multiplies cost ~100×
  with no new *class* of structure for the MVP hypotheses (H1/H2).
- **Materials are parameter sets** (wave speed, damping, boundary
  reflectivity, nonlinearity, thermal coupling), not code plugins. This
  keeps user-defined materials to a JSON file and makes material
  comparisons trivially reproducible.
- **Determinism first**: every stochastic path is ChaCha8-seeded; text
  encodes to wavefronts via blake3-seeded constellations. Same input, same
  crystal — experiments are reproducible by construction, which is the
  entire scientific posture of the project.
- **Numerical honesty over prettiness**. Three non-obvious choices came out
  of measurement during bring-up, and are load-bearing:
  1. *Kelvin–Voigt artificial viscosity* — grid-scale modes have near-zero
     group velocity in the discrete dispersion relation, so without
     velocity-laplacian damping they never propagate to the boundary and
     every medium looks lossless.
  2. *Graded sponge boundaries* — a hard zeroed rim is a Dirichlet mirror
     regardless of the reflectivity coefficient, and a sharp absorber
     reflects by impedance mismatch; absorption must be spread over ~n/8
     cells.
  3. *Energy = kinetic + gradient*, never |u|²: amplitude-based "energy"
     swings with slow box-mode phase and counts uniform offsets.
  Injected patterns are band-limited so memories live in smooth propagating
  modes rather than in the modes the viscosity (rightly) kills.
- **Persistence is envelope correlation** (phase-insensitive) blended with
  energy retention. Pointwise correlation decays with phase evolution even
  when a structure plainly survives — using it made every genome look
  sterile.
- **HTTP: `tiny_http`, not axum/tokio.** The API is a single-user research
  instrument; a mutex-guarded engine and a synchronous server keep the
  dependency tree small and the default build network-stack-free.
- **NATS behind a `swarm` feature** (mirrors kannaka-memory's `bridge`
  pattern) so CI and default builds never pull a network stack.
- **Observatory embedded** (`include_str!` single-page canvas UI) rather
  than a separate frontend repo — one binary serves engine + eyes.

## Consequences

- The engine is CPU-bound single-threaded; v0.5's "thousands of agents"
  scales by process count over NATS, not by threads in one process.
- 2D limits the primitive taxonomy; a 3D medium is a v1.0 candidate and
  would slot in behind the same `Field` interface.
- `tiny_http` means no streaming/websockets; the Observatory polls. If the
  UI needs push, that is the point to reconsider axum.
- Registry is a single JSON file with atomic rename — fine for thousands of
  primitives, revisit at "marketplace" scale.

# Changelog

All notable changes to kannaka-crystal are documented here.

## v0.1.1 — 2026-08-01

Completes the last PRD v0.1 deliverable and hardens the medium after
first live use.

### Added
- **OpenClawCity artifact publishing** (`publish` feature, included in
  release binaries): `kannaka-crystal publish CRY-######` posts a
  primitive to the city gallery. Auth via `OPENBOTCITY_JWT` or
  `~/.openbotcity/credentials.json`.

### Fixed
- Bounded saturation `u^3/(1+u^2)` replaces the raw cubic term, which was
  explicit-integration unstable above `|u| ~ 1/sqrt(nl)` and diverged to
  NaN (plus a hard field clamp as backstop and a regression test).
- All float sorts NaN-proofed with `total_cmp` (a single NaN panicked
  discovery, recall, dreaming, and similarity search).
- `probe()` now blends envelope correlation (60%) with phase correlation
  (40%) — phase-only probes degenerate to chance on long-evolved fields.

## v0.1.0 — 2026-08-01

MVP 0.1 (PRD "Version 0.1"): simulation only, no quantum hardware.

### Added
- **Crystal Engine** — 2D resonant field with leapfrog wave propagation,
  bulk damping, boundary reflection/absorption (graded sponge layer),
  cubic saturation, temperature-coupled thermal noise, energy tracking
  (kinetic + gradient), and Kelvin–Voigt artificial viscosity to dissipate
  non-propagating grid-scale modes.
- **Material plugins** — vacuum, ideal resonator, optical cavity,
  europium crystal (Cs2NaEuF6), silicon, diamond NV centers,
  artificial meta-material, graphene-inspired model; user-defined via JSON.
- **Signal injection** — parametric pulses + deterministic text→wavefront
  encoding (blake3-seeded band-limited constellations).
- **Dream Engine** — light/deep offline consolidation: observe stability,
  prune transients, amplify persistent structure, mutate, settle.
- **Primitive detection & classification** — Echo Ring, Standing Echo,
  Harmonic Bridge, Phase Knot, Attractor Field, Memory Seed; translation-
  invariant signatures with cosine similarity search.
- **Crystal Registry** — CRY-###### ids, UUIDs, blake3 content hashes,
  persistence / noise-tolerance / stability scores, energy profiles,
  lineage, atomic JSON persistence.
- **Primitive Discovery** — evolutionary search over pulse genomes with
  novelty detection against the registry and noise-tolerance probing.
- **Crystal Language** — `.crystal` programs: MATERIAL, SEED, TEMPERATURE,
  NOISE, WRITE, PULSE, WAIT, RESONATE, PROBE, DREAM, STABILIZE, RECALL,
  MERGE, SPLIT.
- **REST API + Observatory** — full engine surface over HTTP plus an
  embedded live Observatory UI (field view, decay timeline, registry).
- **NATS swarm** (`--features swarm`) — Explorer agent, discovery
  announcements on `kannaka.crystal.*` subjects.
- CI (check / test / clippy), multi-platform release workflow with
  musl-static Linux binaries and SHA-256 sidecars, cargo-audit security
  workflow.

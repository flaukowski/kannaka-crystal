# Changelog

All notable changes to kannaka-crystal are documented here.

## v0.5.1 — 2026-08-01

### Fixed
- Swarm agents authenticate with `NATS_USER`/`NATS_PASSWORD` (the
  constellation convention). The swarm server's anonymous user is a
  read-only mirror with a curated subject allowlist that silently drops
  everything else — v0.5.0 fleets could explore but never hear each
  other. First verified live fleet: 3 explorers + archivist, full-mesh
  cross-node merging, Observatory registry 38 → 250+ in minutes.

## v0.5.0 — 2026-08-01

PRD "Version 0.5": distributed swarm exploration. Version number jumps to
match the PRD milestone.

### Added
- **Swarm-aware explorers** — `explore` now merges primitives announced by
  other nodes into its local registry before every search round, so novelty
  detection is swarm-wide and agents stop rediscovering each other's work.
  `--material` accepts a comma-separated rotation list or `all`.
- **Archivist agent** — `kannaka-crystal archive` merges every discovery on
  `kannaka.crystal.primitive.discovered` into this node's registry. Run it
  against the Observatory's data dir and the registry grows live.
- **Cross-node merge semantics** — swarm identity is the UUID; imports get
  a fresh local CRY serial with the origin node + id recorded in
  provenance; near-duplicate structures are rejected like local
  registrations (ADR-0002).
- **Searchable registry** (PRD: "every primitive becomes searchable") —
  CLI: `primitives --class --material --min-persistence --similar CRY-#`;
  API: `GET /primitives?class=&material=&min_persistence=&similar_to=`.
- **Live Observatory registry** — the server reloads the registry file on
  every read and load-modify-saves on writes, so swarm discoveries appear
  in the UI as they land.

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

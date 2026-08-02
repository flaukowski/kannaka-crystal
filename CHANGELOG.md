# Changelog

All notable changes to kannaka-crystal are documented here.

## v0.12.0 — 2026-08-02

ADR-0004 Phase 4.1 (manifest closure): evidence procedures replay a
closed protocol. Found live: replicating CRY-012627 against its own
manifest scored 0.861 (< 0.92) because evolve fitness includes novelty
vs the LIVE registry — re-running a search after its own discoveries
landed takes a different trajectory. Evolve manifests were not closed.

### Changed
- Evolve manifests now record the **genome** behind every registered
  primitive (`results.genomes`, hash-exempt). `replicate`/`perturb`/
  `resolution` replay the primitive's generating simulation directly —
  genome + config fully determine the field evolution, no registry
  state involved. Procedures bumped to `reproduce-v2` /
  `perturbation-ensemble-v2` / `cross-resolution-v2`; records carry
  `method: genome-replay | full-rerun`.
- Pre-4.1 manifests (no genome records) fall back to full-evolution
  replay, which remains registry-coupled and is expected to diverge —
  the record says so instead of pretending.
- Regression test pins the fix: reproduce succeeds after the registry
  grows between runs.

## v0.11.0 — 2026-08-02

ADR-0004 Phase 4 (Behavioral Primitives): capabilities are measured
demonstrations, never morphological inferences.

### Added
- **Behavioral contracts** (`src/behavior.rs`, `behavior-contract-v1`):
  every contract runs the same task WITH the primitive instantiated and
  WITHOUT it, over N deterministic trials — the capability score is the
  mean advantage, so a primitive that doesn't help scores ≈ 0. First two
  contracts: `noise_shielding` (does the primitive's presence protect a
  written memory's physical recall under ambient noise?) and
  `pattern_completion` (given half a target's constellation as a cue,
  does the field recover the full pattern better with the primitive?).
- `kannaka-crystal promote <id> --procedure behavior --capability
  noise_shielding|pattern_completion [--trials 10]` — records a
  `BehavioralCapability` on the primitive (passed or failed — failed
  tests are evidence too). A pass promotes to **Level 6 Behaviorally
  Validated** only if the primitive is already Level ≥ 2 (Replicated):
  a behavior demonstrated on an unreproducible structure demonstrates
  nothing.
- **Capability search**: CLI `primitives --capability <name>`, API
  `GET /primitives?capability=<name>` — matches only PASSED capabilities.
  This is the hook KannakaHDL `capability` queries resolve through.
  Primitive listings show `caps=[…]` for passed capabilities.

## v0.10.0 — 2026-08-02

ADR-0004 Phase 3 (Robustness and Evidence): the evidence ladder lands on
disk, promotion is a recorded procedure, and genealogy becomes actual
ancestry.

### Added
- **Evidence ladder** (§9): every primitive carries `evidence_level`
  (registration = Level 1 Observed; pre-ladder rows default to 1) and
  `evidence_records` — the procedures behind the level. Search gains
  `--min-evidence` / `min_evidence=` (CLI + API): the hook KannakaHDL's
  evidence-floor queries resolve through.
- **Promotion procedures** (`kannaka-crystal promote <id> --procedure …`):
  `replicate` re-runs the exact manifest protocol (Level 2; a FAILED
  replication demotes to Level 1 — negative results are recorded, not
  discarded); `perturb` runs the §8 trial ensemble (shifted seeds ×
  4 noise amplitudes; survival ≥60% → Level 3, and the ensemble survival
  rate replaces the registration-time single-rerun noise_tolerance);
  `resolution` re-runs at 0.75× and 1.5× grid (Level 4).
- **Honest genealogy** (§7): genomes carry stable ids + parentage;
  a primitive's lineage is now primitives registered from its genome's
  parent — the discovery-adjacency pseudo-lineage is gone. Primitives
  record `genome_id` + `parent_genome_ids`.
- `EvolutionConfig.ambient_noise` — perturbation re-runs raise it;
  normal discovery keeps 0.

## v0.9.0 — 2026-08-02

ADR-0004 Phase 2 (Recall and Benchmark Separation): recall channels
split, mechanisms ablatable, and KCB-1 measuring the platform against
baselines built to beat it.

### Added
- **Recall evidence channels** (§3, breaking): `probe()` returns
  `physical_resonance` + its envelope/phase components (field-state
  correlation only); `recall()` returns per-hit physical / encoding /
  semantic / hybrid scores with the hybrid weights versioned
  (`hybrid-v1`: 0.45 physical + 0.15 encoding + 0.40 semantic). Ranking
  uses hybrid; scientific claims use physical alone.
- **Mechanism ablation** (§11): ten switches (damping, nonlinearity,
  viscosity, boundary reflection, thermal/external noise, dream
  pruning/amplification/mutation, semantic recall) on the engine, plus
  the `ABLATE <mechanism> on|off` Crystal Language op so programs carry
  their ablations as protocol.
- **KCB-1 benchmark runner** (§10): `kannaka-crystal bench` — four
  benchmarks (identity recall after delay, rejection of unrelated,
  noise robustness, multi-memory capacity) × five conditions
  (crystal-full, static-encoding, conv-smoothing, no-nonlinearity,
  no-reflection), 10 seeds each, results persisted in the experiment
  manifest. A CI test pins the load-bearing claim: crystal-full must
  hold a positive identity margin after delay.

### First honest baseline (ideal_resonator, 10 seeds, delay 300)
Physical-recall-only results, as the ADR warned, are less impressive
than the old blended scores: static-encoding beats crystal-full on
delayed identity margin (+0.59 vs +0.05) and capacity (0.98 vs 0.65) —
wave mixing erodes content addressing, quantified. Crystal-full's
measured advantage is **distractor rejection** (+0.69, best of all
conditions), and both nonlinearity and reflection ablations degrade it.
Closing the identity-margin gap (dream scheduling, material choice,
delay regimes) is now a measurable research goal instead of a vibe.

## v0.8.0 — 2026-08-02

ADR-0004 Phase 1 (Provenance and Labels): the platform starts grading
its own claims.

### Added
- **Experiment manifests** (§2): every `run` / `evolve` (CLI and API)
  emits an immutable versioned manifest to
  `<data_dir>/experiments/<id>.json` — engine/solver/encoder/dream/
  detector/classifier/signature/fitness versions, material parameters in
  force, environment, seed, program. The **experiment hash** covers the
  protocol only (never results or timestamps), so identical protocols
  hash identically and reproductions are detectable.
- **Experiment provenance on primitives** (§6): new registrations carry
  `experiment_id` + `experiment_hash` alongside the structure hash.
  Pre-manifest rows load unchanged (`None`).
- **Material model classification** (§1): materials declare `model_kind`
  (all builtins: `phenomenological`), `validation_status`
  (`uncalibrated`), `inspired_by`, and `references`. Display names for
  real-substance presets now say "-Inspired" ("Europium-Inspired
  Resonant Medium"); **ids are unchanged** — swarm units and KannakaHDL
  queries keep working.
- **Classification metadata** (§5): every detection carries
  `classifier_version`, a margin-derived `classifier_confidence`, and
  the raw morphology features it was derived from — plus a new
  `Unknown` class instead of forcing weak regions into a named label.
- `src/versions.rs` — the versioned algorithm identifiers.

## v0.7.0 — 2026-08-02

### Added
- **In-browser engine (WASM)** — the full engine core (field, materials,
  pulse, dream, probe, detect) compiles to wasm32 and runs live on the
  Pages site: write text, watch it interfere at 60fps, dream, probe —
  deterministic ChaCha8-seeded, so browser runs replay lab runs exactly.
  Modules split target-wise: filesystem/network surfaces (api, registry,
  discovery, lang, swarm, publish) stay native-only.
- Pages workflow builds the wasm engine with wasm-pack on every deploy;
  wasm-opt disabled for deterministic cross-platform builds.

### Sibling
- **KannakaHDL** lives in its own repo from today:
  https://github.com/flaukowski/kannaka-hdl — the ADR-0003 Part 2
  composition language (grow architectures from this registry's
  primitives), evolving separately.

## v0.6.0 — 2026-08-02

### Added
- **Genealogy tab** in the Observatory (ADR-0003): a Morpho-inspired
  growth replay of the registry — primitives appear in discovery order,
  spawn beside their lineage parents, and a force layout untangles the
  graph. Fill = class, ring = origin swarm node, radius = persistence;
  live mode streams the running swarm's discoveries into the animation.
  Zero dependencies (~200 lines of vanilla canvas JS), view capped at
  500 nodes.
- **`GET /genealogy`** — lean lineage projection (no signatures) with
  origin node parsed from swarm provenance; cheap to poll.
- **ADR-0003** (Proposed): a rewrite-rule composition language over
  cataloged primitives as the H3 architecture layer — base cases are
  registry queries, growth instantiates into a field. Blocked on the
  "ports as resonance couplings" question; documented before syntax.

## v0.5.2 — 2026-08-01

Growth management for long-running fleets (a 3-explorer fleet accumulates
~120 primitives per 10 minutes unbounded).

### Added
- **Registry pruning** — quality-ranked eviction with two env-tunable
  bounds: `KANNAKA_CRYSTAL_BUCKET_CAP` (default 150 per class×material
  bucket, preserving taxonomy diversity) and
  `KANNAKA_CRYSTAL_MAX_PRIMITIVES` (default 5000 total). Applied
  automatically by explorers, the archivist, and the evolve API;
  manually via `kannaka-crystal prune`.
- **Announce floor** — primitives below
  `KANNAKA_CRYSTAL_ANNOUNCE_MIN_PERSISTENCE` (default 0.25) are kept
  locally but never announced to the swarm bus.
- **Explorer pacing** — `explore --interval <secs>` sleeps between search
  rounds so explorers coexist politely on shared boxes.

### Changed
- Release binaries now build with `--features publish,swarm` so fleet
  nodes install straight from release assets.

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

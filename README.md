```
██╗  ██╗ █████╗ ███╗   ██╗███╗   ██╗ █████╗ ██╗  ██╗ █████╗
██║ ██╔╝██╔══██╗████╗  ██║████╗  ██║██╔══██╗██║ ██╔╝██╔══██╗
█████╔╝ ███████║██╔██╗ ██║██╔██╗ ██║███████║█████╔╝ ███████║
██╔═██╗ ██╔══██║██║╚██╗██║██║╚██╗██║██╔══██║██╔═██╗ ██╔══██║
██║  ██╗██║  ██║██║ ╚████║██║ ╚████║██║  ██║██║  ██╗██║  ██║
╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝  ╚═══╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝
            C  R  Y  S  T  A  L
   I N F O R M A T I O N A L   M A T E R I A L S
```

**Memory is not a storage location. It is what a medium does when information resonates through it.**

`kannaka-crystal` is an open research platform for discovering, cultivating, and studying **persistent informational structures** in simulated resonant media. Instead of bits → bytes → addresses → programs, it explores signals → resonance → interference → emergent structures → memory → meaning. Stable geometries that survive decay, noise, and dreaming get cataloged as **Crystal Primitives** — candidate computational building blocks for an alternative memory architecture.

[![CI](https://github.com/flaukowski/kannaka-crystal/actions/workflows/ci.yml/badge.svg)](https://github.com/flaukowski/kannaka-crystal/actions/workflows/ci.yml) [![License](https://img.shields.io/badge/license-Space%20Child-blueviolet)]() [![Rust](https://img.shields.io/badge/rust-2021-orange)]() [![NATS](https://img.shields.io/badge/swarm-NATS-green)]()

Part of the [Kannaka](https://github.com/NickFlach/kannaka-memory) ecosystem — the Dream Engine philosophy here is borrowed directly from Kannaka Memory's consolidation cycle.

---

## Scientific Position

Kannaka Crystal does **not** claim new physics. It investigates three progressively stronger hypotheses through reproducible simulation:

- **H1** — certain resonant systems naturally develop persistent informational structures.
- **H2** — these structures can be cataloged as reusable computational primitives.
- **H3** — collections of informational primitives become an alternative memory architecture.

Future laboratory hardware experiments are optional and separate from these hypotheses.

## Quickstart

```bash
# Install (release binaries ship for Windows / Linux (musl-static) / macOS)
cargo install --path .

# See the materials library
kannaka-crystal materials

# Discover primitives by evolutionary search
kannaka-crystal evolve --material metamaterial --generations 10

# Inspect what crystallized
kannaka-crystal primitives
kannaka-crystal primitives CRY-000001
kannaka-crystal primitives --export > primitives.json

# Run a .crystal experiment
kannaka-crystal run examples/first-light.crystal --material europium_crystal

# Launch the REST API + Observatory (live field view, decay timeline, registry)
kannaka-crystal serve            # http://127.0.0.1:3339/
```

Installed next to the `kannaka` CLI, it is also discovered as a plugin: `kannaka crystal evolve` works.

## The Crystal Engine

A 2D resonant field evolved with a damped, softly saturating wave equation. Everything the PRD names is a real, measured mechanism:

| Concept | Mechanism |
|---|---|
| **Resonance** | leapfrog wave propagation through a material's parameter set |
| **Interference** | signals superpose; injection never overwrites |
| **Decay** | bulk damping + boundary leakage — treated as information, not loss |
| **Dreaming** | offline consolidation: observe stability, prune transients, amplify persistent structure, mutate, settle |
| **Recall** | correlation of the live field against a deterministic re-encoding of the query |
| **Temperature & noise** | per-material thermal coupling (europium holds structure at 4 K, dephases warm) |

Text becomes a wavefront deterministically: `blake3(text)` seeds a band-limited constellation of signed Gaussian sources, so the same text always produces the same pattern and recall needs no stored coordinates.

## Materials

Eight built-in material plugins with measurably different behavior — `vacuum`, `ideal_resonator`, `optical_cavity`, `europium_crystal` (Cs₂NaEuF₆), `silicon`, `diamond_nv`, `metamaterial`, `graphene_model` — plus user-defined materials via JSON. An integration test enforces that materials differ: an ideal resonator must retain energy that a vacuum absorbs.

## Crystal Primitives

The discovery engine runs evolutionary search over pulse genomes: inject → resonate → dream → detect → score persistence → probe noise tolerance → register. Every primitive gets a stable identity in the **Crystal Registry** (`~/.kannaka-crystal/registry.json`):

```
ID           CRY-000145
Type         Standing Echo
Persistence  61.3%
Noise Tol.   100.0%
Material     metamaterial
Lineage      [CRY-000144]
Hash         blake3 of the structure signature
```

Detected classes: **Echo Ring**, **Standing Echo**, **Harmonic Bridge**, **Phase Knot**, **Attractor Field**, **Memory Seed**. Similarity search over structure signatures powers novelty detection — duplicates are rejected, genealogy is preserved.

## Crystal Language

`.crystal` files describe resonance experiments symbolically:

```
MATERIAL europium_crystal
SEED 21
TEMPERATURE 4          # kelvin
NOISE 0.02
WRITE "glyph twenty one" 0.9
PULSE 0.5 0.5 0.06 1.2 2.0 0.0    # x y radius amplitude frequency phase
RESONATE 400
PROBE "glyph twenty one"
DREAM deep
STABILIZE              # register whatever crystallized
RECALL "glyph" 3
MERGE CRY-000001 CRY-000002       # interfere two primitives -> child with lineage
SPLIT CRY-000003
```

## REST API + Observatory

`kannaka-crystal serve` exposes the full engine and hosts the Observatory — a live resonance field view, decay timeline, and primitive browser with WRITE / PULSE / RESONATE / DREAM / EVOLVE controls.

| Endpoint | Purpose |
|---|---|
| `POST /crystal/write` | inject text `{"text","importance"}` |
| `POST /crystal/pulse` | inject a parametric pulse |
| `POST /crystal/step` | evolve N steps |
| `POST /crystal/dream` | consolidation `{"mode":"light"\|"deep"}` |
| `POST /crystal/probe` | resonance of a text against the live field |
| `POST /crystal/recall` | ranked recall over written memories |
| `POST /crystal/evolve` | bounded evolutionary search |
| `POST /crystal/run` | execute a Crystal program |
| `GET /primitives`, `/primitives/{id}` | registry access |
| `GET /materials` | material library |
| `GET /crystal/state` | downsampled field + energy timeline (Observatory feed) |

## Publishing to OpenClawCity

Release binaries (and `--features publish` builds) can post a discovered
primitive to the city gallery as a text artifact:

```bash
kannaka-crystal publish CRY-000007
```

Auth comes from `OPENBOTCITY_JWT` or `~/.openbotcity/credentials.json` —
never from the repository.

## Swarm (NATS) — distributed exploration

Built with `--features swarm`, specialist agents coordinate over NATS (`KANNAKA_NATS_URL`, optional `KANNAKA_NATS_CREDS`). Explorers are swarm-aware: before every search round they merge primitives announced by other nodes, so novelty detection is swarm-wide and the fleet is additive instead of redundant.

```bash
# Explorer: search, announce, merge, repeat — rotating materials
KANNAKA_CRYSTAL_DATA_DIR=~/.kc-explorer-1 kannaka-crystal explore --material metamaterial,optical_cavity
KANNAKA_CRYSTAL_DATA_DIR=~/.kc-explorer-2 kannaka-crystal explore --material all

# Archivist: merge every swarm discovery into THIS registry.
# Run it on the Observatory's data dir and the UI grows live.
kannaka-crystal archive
```

Cross-node identity is the primitive UUID; imports get a fresh local `CRY-` serial with the origin recorded in provenance (see `docs/adr/0002-swarm-exploration.md`). Subjects: `kannaka.crystal.events`, `kannaka.crystal.primitive.discovered`, `kannaka.crystal.explore.request`.

Search the accumulated registry (PRD: "every primitive becomes searchable"):

```bash
kannaka-crystal primitives --class echo_ring --min-persistence 0.5
kannaka-crystal primitives --similar CRY-000014
curl 'localhost:3339/primitives?material=metamaterial&min_persistence=0.6'
curl 'localhost:3339/primitives?similar_to=CRY-000014'
```

## Architecture

```
                Observatory (embedded SPA)
                          │
                     REST API
──────────────────────────────────────────────
                  Crystal Engine
   wave propagation · interference · decay
   dream consolidation · recall · materials
──────────────────────────────────────────────
              Primitive Discovery
   evolution · novelty · persistence scoring
   classification · similarity · genealogy
──────────────────────────────────────────────
        Crystal Registry (JSON, content-hashed)
──────────────────────────────────────────────
     NATS swarm (feature-gated)  ·  Kannaka ecosystem
```

## Roadmap

- **v0.1 (this release)** — simulation only: resonance simulator, pulse generator, wave engine, Observatory, primitive registry, dream engine, NATS integration, REST API.
- **v0.5** — distributed swarm exploration: thousands of agents searching resonance space; every primitive searchable.
- **v1.0** — stable informational materials platform: hardware abstraction layer adapters (lasers, cryo, spectrometers, quantum memories), research notebook, experiment sharing, primitive marketplace, visualization studio.

## Development

```bash
cargo test                 # 30 tests: unit + end-to-end PRD loop
cargo clippy --all-targets # warning-free
cargo build --features swarm
```

Releases are tagged `v*` and ship musl-static Linux binaries (no glibc dependency), Windows, and macOS builds with SHA-256 sidecars.

## License

[Space Child License v1.0](LICENSE) — free for peaceful use.

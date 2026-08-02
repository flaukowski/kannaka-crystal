# ADR-0003: Morpho-Style Growth Views and a Composition Layer for H3

Status: Proposed (genealogy view: Accepted/shipped) · Date: 2026-08-02

## Context

MorphoHDL (paradigms-of-intelligence/morpho, Apache 2.0) is an experimental
HDL where circuits are *grown*: cells carry rewrite rules, recursively
splitting into subcells until LUT base cases, with bus widths inferred at
runtime. Its browser demo renders growth live with a force-directed layout.
It is very young (single-digit commits) — a pattern to borrow, not a
dependency to take.

Two of our problems rhyme with it:

1. The registry's lineage (mutation chains, MERGE children, swarm
   provenance) is a growth process, but we only displayed it as a table.
2. PRD H3 — "collections of informational primitives become an alternative
   memory architecture" — has no language. Crystal Language is an
   experiment script (WRITE / PULSE / RESONATE / STABILIZE): it steers one
   field; it cannot express "an architecture built from primitives."
   The PRD's long-term vision ("what CAD became to mechanical engineering")
   needs that missing layer.

## Decision

**Part 1 (shipped): a Morpho-inspired genealogy view in the Observatory.**
A second tab replays registry growth: primitives appear in discovery
order, spawn beside their lineage parent, and a small force layout
untangles the graph. Fill color = class, ring color = origin node (parsed
from `swarm:<node>:` provenance), radius = persistence. Live mode polls a
new lean `GET /genealogy` endpoint (no 256-float signatures) and streams
the running swarm's discoveries into the animation. Zero external
dependencies — ~200 lines of vanilla canvas JS, view capped at 500 nodes
(the force layout budget), oldest falling off.

**Part 2 (proposed, v1.0): a rewrite-rule composition language over
primitives.** Sketch:

```
# a memory bank grown from cataloged primitives
cell MemoryBank(n):
    when n > 1:  SPLIT -> MemoryBank(n/2), MemoryBank(n/2), bridge: HarmonicBridge
    when n == 1: BASE  -> MemorySeed(min_persistence = 0.6)
```

- **Base cases are registry queries**, not LUTs: `MemorySeed(...)` selects
  a cataloged primitive by class + quality floor (+ material), so grown
  architectures are built from *discovered* structure, never hand-drawn.
- **Growth = staged instantiation into a field**: each grown leaf injects
  its primitive's signature at its laid-out position; bridges couple
  regions. The simulator then answers the H3 question empirically — does
  the composed architecture store and recall better than the sum of its
  parts?
- **Ports are resonance couplings.** The open research problem (and the
  reason this is Proposed, not Accepted): a circuit wire is discrete, a
  coupling between resonant structures is a continuous overlap. The
  port abstraction needs experiments before syntax is frozen.

## Consequences

- The genealogy tab makes swarm structure legible (which primitives are
  generative hubs; how per-node lineages interleave) at no dependency
  cost. Its 500-node cap trades completeness for interactivity; the full
  graph remains queryable via `/genealogy`.
- Part 2 would make Crystal Language a two-layer system (experiments
  below, architectures above). It should not start until the port
  question has empirical footing — premature syntax here would freeze the
  wrong abstraction.
- If MorphoHDL matures, its compiler's SoA layout and renderer are worth
  revisiting; today we take only the ideas.

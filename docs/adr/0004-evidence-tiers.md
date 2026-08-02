# ADR-0004: Evidence Tiers — Preserving Epistemic Distinctions

Status: Proposed · Date: 2026-08-02

## Context

The platform is exploratory, but exploration must be paired with explicit
evidence. The founding distinctions (from the project's scientific
position):

> A structure may be beautiful before it is useful.
> A behavior may be useful before it is physically validated.
> A material model may be informative before it represents a real
> substance. The platform will preserve those distinctions and provide a
> path by which generated structures can become reproduced primitives,
> validated behaviors, calibrated models, and eventually physical
> informational materials.

Today the registry flattens all of this: a Standing Echo detected once in
one seed carries the same implicit confidence as one measured across
dozens of runs. Persistence and noise-tolerance numbers exist, but
nothing records *what kind of claim* a row is entitled to make.

## Decision (proposed)

**A `evidence` tier on every primitive, promoted only by explicit,
recorded procedure — never by time or popularity.**

| tier | claim | promotion gate |
|---|---|---|
| `generated` | "this geometry emerged once" | default at registration — today's entire registry |
| `reproduced` | "it emerges again on demand" | re-grow from the stored genome/seed on a *different* node or build; signature similarity ≥ threshold. The engine is deterministic by construction, so this verifies the *pipeline*, not luck. |
| `validated` | "it does something useful, with error bars" | a named behavior (e.g. recall boost, noise shielding) measured across ≥10 seeds (10-run minimum — 5 is not enough) with mean ± spread recorded |
| `calibrated` | "its material model tracks a real substance" | material parameters fitted against published constants of the named substance; fit residuals recorded |
| `physical` | "it exists in matter" | realized through the Hardware Abstraction Layer (v1.0+); instrument data attached |

- Each promotion appends an **evidence record** (procedure, inputs,
  numbers, date, node) to the primitive — the tier is a summary of its
  records, and demotion is possible when a record fails replication.
- **Nothing existing is grandfathered**: on migration every current
  primitive is `generated`. The swarm's 3,000+ discoveries are exactly
  that — generated structures, honestly labeled.
- Search, KannakaHDL resolution, and announcements carry the tier, so an
  architecture can require `base "Standing Echo" min_evidence reproduced`
  and the bus can prioritize higher-evidence announcements.
- Beauty stays first-class: `generated` is not a deprecation. It is the
  tier where discovery lives; the ladder exists so nothing has to
  masquerade as more than it is.

## Consequences

- Registry schema gains `evidence: string` + `evidence_records: []`
  (backward-readable: absent field = `generated`). This touches the
  kannaka-hdl contract (ADR-0002's coupling surface) — hdl should treat a
  missing tier as `generated` and gain `min_evidence` in queries.
- A `kannaka-crystal reproduce CRY-######` command becomes the first
  promotion tool (re-grow from stored provenance); `validate` follows
  with the 10-run behavior harness. Both are v0.8 candidates.
- The Observatory and genealogy views can render tier (e.g. ring
  thickness), making the epistemic state of the catalog visible at a
  glance.

## Note

Drafted from the closing section of the updated scientific-position text;
the source arrived truncated. If the full document contains additional
tiers or commitments, this ADR should be aligned to it before acceptance.

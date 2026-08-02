# ADR-0004: Establish a Research Validity and Primitive Evidence Model

- **Status:** Proposed
- **Date:** 2026-08-02
- **Decision Owners:** Kannaka Crystal maintainers
- **Applies To:** Crystal Engine, Crystal Foundry, Crystal Registry, Crystal Observatory, swarm explorers, experiment emitters
- **Related:** ADR-0002 Swarm Exploration, ADR-0003 Morpho Composition Layer, KannakaHDL ADR-0002

## Context

Kannaka Crystal has progressed from a conceptual product specification
into an operational research platform. It now includes:

- A deterministic resonant-field simulation engine
- Phenomenological material presets
- Pulse and text-to-wavefront encoding
- Dream consolidation
- Primitive detection and classification
- Evolutionary search
- Noise-tolerance testing
- Persistent primitive registration
- NATS-based distributed exploration
- Genealogy visualization
- REST, CLI, WASM, and Observatory interfaces

The system can generate persistent structures, classify them, distribute
discoveries across a swarm, and compose them into larger experimental
architectures.

This progress introduces a new risk: **the platform can produce
compelling results faster than it can establish what those results
mean.**

The current system sometimes combines several different layers of
interpretation:

1. Numerical behavior produced by the Crystal Engine
2. Heuristic descriptions of observed morphology
3. Memory and recall behavior
4. Material-inspired presets
5. Claims about physical or quantum systems
6. Product-facing language used by the Observatory and documentation

Without an explicit evidence model, users may interpret a persistent
field structure as a validated memory primitive, a material preset as a
physical simulation, or a morphological label as a discovered natural
category.

The platform must therefore evolve from a system that discovers
interesting structures into an instrument capable of testing,
reproducing, rejecting, and grading its own discoveries.

## Decision

Kannaka Crystal will adopt a formal **Research Validity and Primitive
Evidence Model**.

Every experiment, primitive, material model, classifier output, and
memory result will carry enough metadata to distinguish:

- What was directly observed
- What was inferred
- What was heuristically classified
- What was reproduced
- What was validated against alternative models
- What remains speculative

The following changes are adopted as the foundation of this model.

### 1. Material Models Will Be Explicitly Classified

Kannaka Crystal will distinguish among four material-model categories.

**Phenomenological Model** — a parameter configuration for the generic
Crystal Engine designed to exhibit a useful qualitative behavior.
Examples include current presets inspired by europium crystals, silicon,
diamond nitrogen-vacancy centers, graphene, optical cavities, and
metamaterials. Phenomenological models do not claim to reproduce the
actual governing physics of the named material.

**Physical Model** — a model implementing equations associated with a
documented physical mechanism. Examples may eventually include optical
Bloch equations, photon-echo protocols, atomic frequency comb behavior,
inhomogeneous broadening, spin-ensemble dynamics, and coupled-mode
optical systems.

**Calibrated Model** — a physical or phenomenological model whose
parameters have been fitted to published or experimentally measured
data.

**Hardware Profile** — a model and operating envelope associated with a
specific laboratory apparatus or physical sample.

Each material definition will eventually include fields similar to:

```json
{
  "model_kind": "phenomenological",
  "validation_status": "uncalibrated",
  "display_name": "Europium-Inspired Resonant Medium",
  "inspired_by": ["Cs2NaEuF6 rare-earth optical memory systems"],
  "references": [],
  "calibration_dataset": null
}
```

Current material presets will be treated as phenomenological unless
explicitly upgraded through evidence. User-facing documentation may
retain recognizable material names, but must identify them as inspired
models rather than validated physical simulations.

### 2. Every Experiment Will Produce an Immutable Manifest

Every Crystal experiment will produce a machine-readable experiment
manifest:

```json
{
  "schema_version": "1",
  "experiment_id": "UUID",
  "started_at": "ISO-8601",
  "engine_version": "0.8.0",
  "git_commit": "SHA",
  "solver_version": "leapfrog-v1",
  "field_size": 128,
  "seed": 42,
  "material": {
    "id": "europium_inspired",
    "model_kind": "phenomenological",
    "parameters": {}
  },
  "environment": {
    "temperature_k": 4.0,
    "noise_amplitude": 0.02
  },
  "encoding_version": "text-wavefront-v1",
  "dream_version": "dream-v1",
  "detector_version": "connected-region-v1",
  "classifier_version": "morphology-v1",
  "signature_version": "signature-16x16-v1",
  "fitness_version": "fitness-v2",
  "program": {},
  "results": {},
  "artifacts": [],
  "parent_experiments": []
}
```

An experiment must be reproducible from its manifest unless it is
explicitly marked as dependent on nondeterministic or external hardware
behavior. A registry primitive will reference the experiment manifest
that produced it.

### 3. Recall Will Be Separated Into Distinct Evidence Channels

Kannaka Crystal will no longer present a single blended recall score as
evidence of resonant memory. Recall will be reported through distinct
channels:

- **Physical Recall** — correlation between the current medium state and
  an encoded target pattern.
- **Encoding Recall** — similarity produced by the deterministic encoder
  independently of field evolution.
- **Semantic Recall** — lexical, symbolic, embedding, or other semantic
  similarity applied outside the physical field.
- **Hybrid Recall** — a configurable combination of physical, encoding,
  and semantic scores.

Example result:

```json
{
  "physical_resonance": 0.18,
  "encoding_similarity": 0.42,
  "semantic_similarity": 0.76,
  "hybrid_score": 0.61
}
```

Scientific reports and benchmarks will use physical recall independently
before considering hybrid retrieval. User-facing applications may use
hybrid recall, but must expose or preserve its component scores.

### 4. Primitive Fitness Will Expand Beyond Persistence

Persistence and retained energy are necessary properties, but they do
not demonstrate useful memory or computation. The Crystal Foundry will
evolve toward a multidimensional fitness model including: persistence,
noise tolerance, input separability, recoverability, partial-cue
completion, multi-memory capacity, resistance to catastrophic
interference, energy efficiency, novelty, and reproducibility.

A target fitness structure may resemble:

```text
fitness =
    0.20 persistence
  + 0.15 noise tolerance
  + 0.15 separability
  + 0.15 recoverability
  + 0.15 partial-cue completion
  + 0.10 capacity
  + 0.05 energy efficiency
  + 0.05 novelty
```

Weights will be versioned and configurable. **A primitive must not be
described as a memory primitive solely because it persists.**

### 5. Primitive Classification Will Separate Morphology From Behavior

The existing classes (Echo Ring, Standing Echo, Harmonic Bridge, Phase
Knot, Attractor Field, Memory Seed) are useful observational labels
arising from morphological heuristics (area, elongation, annularity,
angular occupancy, stability density, region size). They will be
formally identified as **Morphological Primitive Classes**.

Each classification will include:

```json
{
  "display_class": "Phase Knot",
  "primitive_domain": "morphological",
  "classifier_version": "morphology-v1",
  "classifier_confidence": 0.64,
  "features": {
    "relative_area": 0.013,
    "elongation": 1.41,
    "annularity": 0.19,
    "angular_gap_count": 4,
    "stability_ratio": 2.8
  }
}
```

The system will support an `Unknown` / `Unclassified` result rather than
forcing every structure into a named class.

A second ontology will be developed for **Behavioral Primitive
Classes**, potentially including: Delay Line, Noise Filter, Pattern
Completer, Frequency Selector, Phase Inverter, Associative Binder,
Persistent Oscillator, Signal Router, Collision Gate, Memory Cell,
Resettable Latch. A structure may carry both a morphological class and
one or more behaviorally demonstrated capabilities.

### 6. Primitive Identity Will Include Structure and Experiment Provenance

A primitive will have at least two independent hashes:

- **Structure Hash** — derived from the normalized observed structure or
  signature.
- **Experiment Hash** — derived from the complete generating protocol,
  model, solver, environment, seed, and versioned algorithms.

This allows the registry to distinguish: the same structure produced
through different mechanisms; different structures produced by the same
genome; similar structures observed under different models;
reproductions of the same experiment; structures that remain stable
across solver changes. Primitive identities will not be based only on a
display class or final downsampled image.

### 7. Genealogy Will Represent Actual Ancestry

Primitive genealogy will no longer use discovery order as a substitute
for genetic parentage. Genomes will receive stable identities and
ancestry metadata:

```json
{
  "genome_id": "UUID",
  "parent_genome_ids": ["UUID"],
  "source_primitive_ids": ["CRY-000145"],
  "mutation_operations": [],
  "generation": 12
}
```

A primitive may expose several distinct relationships: produced by
genome; genome parentage; derived from primitive; structurally similar
to primitive; chronologically preceded by primitive; composed from
primitives; replicated from experiment.

**The Observatory must not present chronological adjacency as biological
or algorithmic descent.**

### 8. Noise and Robustness Metrics Will Use Trial Ensembles

Noise tolerance will be measured across repeated trials and multiple
noise levels rather than from a single rerun. A minimum robustness
profile will include: multiple deterministic seeds, multiple noise
amplitudes, mean persistence, standard deviation, worst-case
persistence, failure rate, and area under the noise-response curve.

Example protocol:

```text
seeds: 8
noise amplitudes: [0.005, 0.01, 0.02, 0.05]
```

The registry may retain a summary score, but the underlying trial
distribution must remain accessible.

### 9. Discoveries Will Be Graded Through an Evidence Ladder

Every primitive and experimental claim will carry an evidence level.

| level | name | meaning |
|---|---|---|
| 0 | Generated | a structure was produced by one run |
| 1 | Observed | passed the detector and minimum registration requirements |
| 2 | Replicated | reproduced using the same protocol across multiple seeds or runs |
| 3 | Perturbation-Stable | survived defined noise and parameter perturbations |
| 4 | Resolution-Stable | survived changes in grid size or simulation resolution |
| 5 | Cross-Solver Stable | survived implementation with an alternate solver or integration method |
| 6 | Behaviorally Validated | repeatedly performed a defined information-processing function |
| 7 | Physically Calibrated | the underlying model was calibrated against published or experimental physical data |
| 8 | Hardware Observed | the structure or behavior was measured in a physical apparatus |

**A lower evidence level is not a failure. It describes the current
strength of the result.**

### 10. Kannaka Crystal Will Establish a Formal Benchmark Suite

The project will create a versioned benchmark suite, initially named
**KCB-1: Kannaka Crystal Benchmarks, Version 1**, including:

1. Identity recall after delay
2. Rejection of unrelated inputs
3. Partial-cue completion
4. Noise robustness
5. Multi-memory capacity
6. Catastrophic interference
7. Sequential learning
8. Temporal ordering
9. Compositional recall
10. Cross-seed reproducibility
11. Cross-grid stability
12. Ablation against non-resonant baselines

Candidate comparison baselines include: static deterministic encoding,
nearest-neighbor retrieval, ordinary autocorrelation, random reservoirs,
echo-state networks, Hopfield networks, convolutional smoothing, field
evolution without dreaming, dreaming with shuffled stability maps, field
evolution without nonlinearity, field evolution without reflection.

**The benchmark must be capable of showing that a Crystal mechanism does
not add value.**

### 11. Core Mechanisms Will Support Ablation

The following mechanisms will be individually disableable through
experiment configuration: damping, nonlinearity, saturation, boundary
reflection, artificial viscosity, dreaming, mutation, novelty scoring,
semantic recall, thermal noise, external noise, structure amplification,
transient pruning. This will allow the project to determine which
mechanisms are responsible for an observed effect.

### 12. The Registry Will Evolve Toward Primitive Contracts

A registered primitive will eventually include more than morphology and
persistence. The target primitive contract will contain: identity,
provenance, morphology, behavioral capabilities, replay protocol,
operating envelope, material requirements, scale constraints, input
sensitivity, output behavior, coupling profile, composition history,
evidence level, benchmark results.

This contract will serve as the bridge between Kannaka Crystal,
KannakaHDL, Kannaka Memory, and the wider swarm.

## Consequences

**Positive**

- Scientific and product claims become easier to distinguish.
- Experiments become reproducible and comparable.
- The registry becomes useful as a genuine research dataset.
- Behavioral discoveries become more important than visually interesting structures.
- KannakaHDL gains a stronger foundation for composition.
- Material names can remain evocative without implying unsupported physical fidelity.
- The swarm can search against explicit capability and evidence requirements.
- Failed hypotheses and negative results become valuable outputs.

**Negative**

- Registry and experiment schemas become more complex.
- Some currently registered primitives may be downgraded or reclassified.
- Existing genealogy views may require migration.
- Benchmarks and repeated trials will increase computation time.
- Results may initially appear less impressive when semantic boosts and heuristics are separated.
- Historical primitive identities may require version-aware interpretation.

**Neutral**

- Existing morphology classes remain useful as display and exploration labels.
- Existing phenomenological material presets remain useful.
- The current solver remains valid as the first Crystal Engine implementation.
- This ADR does not require immediate implementation of quantum or hardware models.

## Non-Goals

This ADR does not: claim that Kannaka Crystal models quantum memory;
require immediate replacement of the current field solver; require
laboratory hardware; eliminate poetic or exploratory naming; prevent
hybrid semantic retrieval in product applications; require all
experiments to reach high evidence levels; define the final form of a
physical rare-earth memory model; define the final coupling-port model
for KannakaHDL.

## Implementation Sequence

**Phase 1: Provenance and Labels** — experiment schema versioning;
solver/encoder/detector/classifier/fitness versions; material model
category; structure and experiment hashes; explicit morphological
classification metadata.

**Phase 2: Recall and Benchmark Separation** — split physical, encoding,
semantic, and hybrid recall; create KCB-1 benchmark runner; add core
ablation flags; persist benchmark results in experiment manifests.

**Phase 3: Robustness and Evidence** — multi-seed noise trials; evidence
ladder; cross-resolution validation; correct genome and primitive
genealogy.

**Phase 4: Behavioral Primitives** — behavioral test contracts; register
demonstrated capabilities; behavioral search in the registry; expose
capabilities to KannakaHDL.

**Phase 5: Physical Models** — phenomenological/physical/calibrated/
hardware model namespaces; references and calibration metadata; physical
models only when supported by documented mechanisms or data.

## Acceptance Criteria

This ADR will be considered implemented when:

1. Every new experiment can emit a complete versioned manifest.
2. Material presets declare their model category and validation status.
3. Recall results expose physical, encoding, semantic, and hybrid components.
4. Primitive records reference both structural and experimental provenance.
5. Noise tolerance uses repeated trials.
6. Genealogy reflects actual genome or composition ancestry.
7. KCB-1 can compare Crystal against at least three non-Crystal baselines.
8. Primitive classifications identify their classifier version and morphology features.
9. At least one behavioral primitive capability can be tested and registered.
10. KannakaHDL can query primitive evidence or behavioral capability through the registry.

## Decision Summary

Kannaka Crystal will remain an exploratory system, but exploration will
be paired with explicit evidence.

A structure may be beautiful before it is useful.

A behavior may be useful before it is physically validated.

A material model may be informative before it represents a real
substance.

The platform will preserve those distinctions and provide a path by
which generated structures can become reproduced primitives, validated
behaviors, calibrated models, and eventually physical informational
materials.

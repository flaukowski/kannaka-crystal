# ADR-0002: Distributed Swarm Exploration (PRD v0.5)

Status: Accepted · Date: 2026-08-01

## Context

PRD Version 0.5: "Thousands of agents search resonance space
simultaneously. Every primitive becomes searchable." v0.1 shipped a single
self-scheduling Explorer that announced discoveries on NATS but never
listened — two explorers would happily rediscover each other's primitives
forever, and each node's registry was an island.

## Decision

**Registry stays per-node JSON; the swarm converges through NATS
announcements, not a shared database.**

- **Identity across the swarm is the primitive's UUID.** CRY-###### serials
  are per-node display handles and WILL collide between nodes, so an
  imported primitive gets a fresh local serial; the origin node and its
  serial are recorded in provenance
  (`swarm:<node>:<remote-id> | <original provenance>`). Lineage strings are
  kept verbatim as a genealogical record — they resolve on the origin node,
  not necessarily locally.
- **Dedup on import = dedup on registration**: reject if the UUID is
  already present or a same-class structure matches at signature
  similarity ≥ 0.92. This is what makes the fleet additive — an explorer
  that merges the swarm's discoveries before each round treats them as
  prior art and its novelty search pushes elsewhere.
- **Two agent roles, one binary**: `explore` (search + announce + merge)
  and `archive` (merge-only). The intended topology on one machine is N
  explorers with private data dirs (`KANNAKA_CRYSTAL_DATA_DIR`) plus one
  archivist on the Observatory's data dir. Explorers never share a
  registry file — coordination is entirely bus-mediated.
- **The Observatory reads through the file**: the server reloads
  `registry.json` on every read and load-modify-saves on its own writes.
  There is no cross-process file lock; the archivist and server both use
  atomic rename, so a racing write loses an update but never corrupts the
  file. Acceptable at v0.5 write rates; a lock (or single-writer rule)
  is the known upgrade if merge rates grow.
- **Scale-out unit is the process, not the thread.** The engine is
  single-threaded on purpose (ADR-0001); "thousands of agents" means
  thousands of `explore` processes across machines sharing one NATS
  subject space, each cheap and independently killable.

## Consequences

- A network partition just slows convergence: nodes keep exploring and
  merge what they missed when announcements resume — but announcements are
  fire-and-forget (no replay), so a node that was offline misses those
  primitives until they resurface through some future node's announcement.
  A registry sync/anti-entropy protocol is the v1.0 candidate fix.
- Signature-based dedup at 0.92 means two genuinely distinct-but-similar
  structures found on different nodes keep only the first arrival locally.
  That is the same tradeoff local registration already makes.
- The anonymous-publish posture of the constellation NATS applies here:
  announcements are unauthenticated data. Merged primitives are inert
  registry rows (no code, no paths), so the blast radius of a forged
  announcement is registry noise; signing/verification would ride on the
  kannaka-memory provenance substrate if needed.

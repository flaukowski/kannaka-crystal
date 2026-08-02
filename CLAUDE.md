# Claude Code Configuration — kannaka-crystal

Wave-interference research platform: discovering persistent informational
structures (Crystal Primitives) in simulated resonant media. See
`docs/adr/0001-mvp-architecture.md` for the load-bearing design decisions.

## Rules

- ALWAYS read a file before editing it
- NEVER commit secrets, credentials, or .env files (NATS creds come from
  `KANNAKA_NATS_URL` / `KANNAKA_NATS_CREDS` env only)
- CI enforces `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`
  — run both before committing
- The `swarm` feature is NOT default; `cargo check --features swarm --all-targets`
  must stay green (CI gates it)
- Numerical changes to `src/field.rs` (viscosity, sponge, energy metric) are
  load-bearing — the integration tests in `tests/end_to_end.rs` encode why;
  read ADR-0001 before "simplifying" them
- NEVER `kannaka-crystal run` experiments against the live data dir while
  swarm agents (archivist/explorers) are running — registry writes race
  (last-writer-wins, confirmed live). Copy registry.json to a scratch dir
  and set `KANNAKA_CRYSTAL_DATA_DIR` (ADR-0002)

## Quick Reference

```bash
cargo test                                   # 30 tests, all must pass
cargo run -- serve                           # Observatory at :3339
cargo run -- evolve --material metamaterial  # discover primitives
cargo run -- run examples/first-light.crystal
```

- Data dir: `~/.kannaka-crystal` (env: `KANNAKA_CRYSTAL_DATA_DIR`)
- Releases: tag `v*` → multi-platform binaries + SHA-256 sidecars

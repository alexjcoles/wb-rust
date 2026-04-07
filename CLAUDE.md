# Wynn Build Advisor

## Project Overview
AI-powered Wynncraft build optimiser. Rust backend + React frontend.
See `wynn-build-advisor-plan.md` for full architecture and design.

## Workspace
Cargo workspace with 4 crates: wynn-core, wynn-encoding, wynn-solver, wynn-api.
React frontend in `frontend/` (not yet created).

## Key upstream references
- WynnBuilderTools (Rust, AGPL-3.0): item DB loading, SP validation, stat calcs, URL encoding
- WynnBuilder (JS, GPL-3.0): ENCODING.md (URL hash format), stat formulas
- Item data: fetched at runtime from hppeng-wynn GitHub repo

## Conventions
- Use `thiserror` for error types
- Use `serde` for all serialization
- Use `axum` for HTTP (not actix)
- Use `reqwest` for HTTP client
- Item data loaded once at startup into `Arc<ItemDb>`
- All stat calculations must match WynnBuilder exactly
- Encoding/decoding uses the V12 binary format (latest)

## Testing
- `cargo test --workspace` to run all tests
- Round-trip tests for encoding: decode(encode(build)) == build
- Stat calculation tests: compare against known WynnBuilder outputs
- Solver tests: verify constraint satisfaction on results

## Running
- Backend: `cargo run -p wynn-api`
- Tests: `cargo test --workspace`

# Wynn Build Advisor

AI-powered build optimiser for [Wynncraft](https://wynncraft.com/). Paste a
[WynnBuilder](https://hppeng-wynn.github.io/builder/) URL, describe what you
want to change ("improve survivability", "fix my thunder defence"), and get
back modified builds with explanations.

## Stack

- **Rust** Cargo workspace (4 crates):
  - `wynn-core` — item DB, stats, skill-point assignment
  - `wynn-encoding` — V12 binary URL hash encode/decode
  - `wynn-solver` — constrained search over the item DB
  - `wynn-api` — axum HTTP server + AI orchestration
- **React + TypeScript** frontend (Vite)
- **Claude** for natural-language reasoning (via the `claude` CLI)

## What's interesting

- Reimplements WynnBuilder's exact skill-point assignment algorithm
  (recursive permutation search with pop-off detection) so totals match the
  reference tool exactly
- Round-trippable V12 binary URL codec, including ability-tree and aspect
  passthrough so modified builds keep the player's tree
- Staged solver pipeline (diverse candidate selection → cheap stat filters →
  tight SP check → full calc) with progressive widening when no results are
  found in time
- Parallel enumeration with `rayon` and a shared deadline flag

## Running

```sh
cargo run -p wynn-api          # backend on :5656
cd frontend && npm run dev     # frontend on :5173
./dev.sh                       # both at once
cargo test --workspace         # tests
```

## Licence

AGPL-3.0 (inherited from WynnBuilderTools, which the solver design draws on).

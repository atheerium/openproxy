# CipherRoute Architecture

Single Rust binary (`src/`) serving both an OpenAI-compatible proxy API and the Astro dashboard (`web/`).

## Layout

```
src/                  # Rust crate (lib.rs + main.rs)
  cli/                # CLI tool subcommands
  core/
    combo/            # routing: combos, capabilities, capacity adapter, ordering
    executor/         # ProviderExecutor trait + default/specialized impls
    translator/       # request/response format translation registry
    usage/            # usage tracking, pricing, quota fetchers
    media/tts/        # text-to-speech executors
  db/                 # SQLite WAL persistence, encrypted columns
  oauth/              # token refresh, background refresh
  server/             # Axum HTTP server, API routes, auth, state
  types/              # shared data types (UsageDb, UsageEntry, etc.)
web/                  # Astro dashboard
  src/pages/          # routes: /dashboard/* , /login , /
  src/components/     # UI components
  src/shared/         # constants, models, utils, store
  src/lib/            # astro lib
  dist/               # built output served by Rust binary
```

## Request flow (API)

```
client → src/server/api/chat.rs:forward_with_provider_fallback
  → format detection → combo resolution (src/core/combo/mod.rs)
  → capability-aware ordering → provider execution (src/core/executor/)
  → response translation (src/core/translator/)
  → SSE / JSON stream back to client
  → usage recorded (src/core/usage/tracker.rs → src/types/mod.rs UsageDb.history)
```

## Dashboard flow

```
browser → web/dist/* (served by src/server/)
  → API calls to /api/* (src/server/api/)
  → SQLite (src/db/) for config + usage history
```

## Key data types

| Type | File | Purpose |
|---|---|---|
| `UsageEntry` | `src/types/mod.rs` | one request log row |
| `UsageDb` | `src/types/mod.rs:805` | `history: Vec<UsageEntry>`, `daily_summary: BTreeMap<String, DailySummary>`, `total_requests_lifetime` |
| `DailySummary` | `src/types/mod.rs:897` | per-day aggregation (has `.requests` field) |
| `ProviderConfig` | `src/core/executor/default.rs` | provider endpoint + auth config |
| `ComboStrategy` | `src/core/combo/mod.rs` | round-robin / fallback / balanced |

See `docs/ROUTING.md` for the routing engine truth and `docs/PROVIDERS.md` for provider registration.

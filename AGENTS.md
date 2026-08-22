# OpenProxy — Rust AI Proxy Router

## What
OpenProxy is an AI proxy router written in Rust — OpenAI-compatible endpoint that routes requests to 40+ AI providers with format translation, account fallback, token refresh, usage tracking, and SSE streaming.

## Why
Replace 9router (Node.js) with a faster, safer Rust implementation that avoids 235+ bugs found in the JS version. Critical patterns: type-safe format handling, encrypted secrets, immutable data flow, thread-safe by design.

## How (Architecture)
- **Core**: model parsing → format detection → request translation → provider execution → response translation → SSE streaming
- **Account mgmt**: credential selection → token refresh → model-level fallback → combo/fusion
- **Executor trait**: `ProviderExecutor` with default+specialized impls
- **Persistence**: SQLite WAL + encrypted columns + usage tracking
- **Security**: HMAC API keys, bcrypt auth, SSRF protection

## Beads
Parity work: epic `openproxy-9router-parity-v0550-pnc` (9router v0.5.50 → openproxy, 122 specs) (+ children). Prior v0.5.30 epic `openproxy-9router-parity-mj1` is closed. See `br ready` / `bv --robot-next`.

## Key References
- `docs/parity-9router.md` — intentional divergences, pipeline order, executor dispatch
- 9router reference: `/tmp/9router` (open-sse) — do NOT copy JS bugs blindly
- **OmniRoute v3.8.50** (`/tmp/omniroute_v3850`, SHA `6cd4d38`) — authoritative reference for provider parity. When a provider behaves unexpectedly, compare against OmniRoute's `src/managed/` implementations, not the legacy JS `9router`. Key files: `credentialHealth.ts` (`HealthStatus`: 200/401/429/503/500), `healthCheck.ts`, `modelDiscovery.ts` (`discoverProviderModels` — GET `/v1/models` with provider-specific headers), `managedModelImport.ts` (merge/sync remote catalog → local config, `preserveRemovedCustomModelCompat`). Header-fidelity rules for parity: OpenRouter must send `HTTP-Referer` + `X-Title`; nvidia/llm7 omit `HTTP-Referer`; gemini sends `x-goog-api-key`; kiro/opencode/free-providers use the appropriate OAuth token flow. Always check `/tmp/omniroute_v3850` before making provider-specific decisions.

## Dev Workflow — single script

One script builds, tests, and starts everything — **quick is the default** and it auto-starts the server:

```bash
./scripts/dev.sh                 # quick (default): cargo build + web build + quick tests + start detached on :4623
./scripts/dev.sh --full          # + full lib suite (1690 tests, --test-threads=1)
./scripts/dev.sh --no-web        # skip dashboard build
./scripts/dev.sh --no-test       # skip tests
./scripts/dev.sh --no-run        # build+test only, don't start server
./scripts/dev.sh --foreground    # start foreground (Ctrl+C to stop) instead of detached
PORT=4624 ./scripts/dev.sh       # custom port
```

Backend (`cargo build --bin openproxy`) and dashboard (`web/src` → `web/dist` via Astro) are separate builds served by the same binary. The default `quick` tests run `provider_models` (13) + `import_catalog` (4) + `parity_tests`; `--full` runs the full `cargo test -p openproxy --lib --test-threads=1`. Dashboard `web/dist` is what the Rust server serves — `dev.sh` rebuilds it for you so features don't appear "missing" (past "feature not found" confusion was a missing `pnpm build`).

For live dashboard iteration without rebuilding:

```bash
pnpm --dir web dev               # Astro dev on :4624 (proxies API to :4623)
```

## Status
Active parity port. Run `cargo test -p openproxy --lib parity_tests stream_flags` for smoke.

## Local Config & Secrets — Never Commit
- **Do not commit** local user config or secrets: `opencode.json`, `.env`, `.env.*`, `*.pem`, `~/.openproxy/db.json`, `~/.openproxy/admin.key`, API keys, `provider_specific_data` with live credentials, or any file containing `sk-`, `Bearer`, `refresh_token`.
- `opencode.json` is local agent config (model, MCP keys like `CONTEXT7_API_KEY`, permissions) — keep untracked. `scripts/dev.sh` builds locally; real secrets live in SQLite (`db.json` encrypted) + `OPENPROXY_API_KEY` env, not in git.
- Before `git add`/`commit`, run `git status` and `git diff --cached`; if a file contains secrets or is machine-local, `git restore --staged <file>` and add it to `.gitignore`. Prefer `git check-ignore -v <file>` to verify.
- If a secret is accidentally committed, rotate it immediately and purge history (`git filter-repo` or BFG) — do not just revert.

## Schema stability (`openproxy.v1.*`)

The `openproxy.v1.*` envelope namespace is a **frozen, additive-only contract**. Every JSON envelope emitted by `--robot` carries a `schema` field matching `openproxy.v1.<area>.<action>`. Existing fields keep their names, types, and meanings across releases. New fields are additive only — no renames or removals. A new `openproxy.v2.*` namespace will be opened before any breaking change.

Run `openproxy schema stability` to see the current stability promise:

```bash
openproxy --robot schema stability
# → {"schema":"openproxy.v1.schema.stability","data":{"namespace":"openproxy.v1","stability":"stable","policy":"..."}}
```

The `schema` subcommand provides four operations:

| Command | Purpose |
|---|---|
| `openproxy schema list` | List all resource kinds with schema and example support |
| `openproxy schema show <resource>` | Print JSON Schema for a resource (provider, key, combo, etc.) |
| `openproxy schema example <resource>` | Print an example payload for a resource |
| `openproxy schema stability` | Print the v1 namespace stability contract |

13 resources are covered: `provider`, `provider-node`, `combo`, `key`, `pool`, `settings`, `custom-model`, `model-alias`, `usage-event`, `log-event`, `chat-event`, `quota`, `oauth-status`. Each has both a schema and an example — enforced by tests.

# CipherRoute — Rust AI Proxy Router

## What
CipherRoute is an AI proxy router written in Rust — OpenAI-compatible endpoint that routes requests to 40+ AI providers with format translation, account fallback, token refresh, usage tracking, and SSE streaming.

## Why
Replace 9router (Node.js) with a faster, safer Rust implementation that avoids 235+ bugs found in the JS version. Critical patterns: type-safe format handling, encrypted secrets, immutable data flow, thread-safe by design.

## How (Architecture)
- **Core**: model parsing → format detection → request translation → provider execution → response translation → SSE streaming
- **Account mgmt**: credential selection → token refresh → model-level fallback → combo/fusion
- **Executor trait**: `ProviderExecutor` with default+specialized impls
- **Persistence**: SQLite WAL + encrypted columns + usage tracking
- **Security**: HMAC API keys, bcrypt auth, SSRF protection

## Beads
Parity work: epic `cipherroute-9router-parity-v0550-pnc` (9router v0.5.50 → cipherroute, 122 specs) (+ children). Prior v0.5.30 epic `cipherroute-9router-parity-mj1` is closed. See `br ready` / `bv --robot-next`.

## Key References
- `docs/parity-9router.md` — intentional divergences, pipeline order, executor dispatch
- 9router reference: `/tmp/9router` (open-sse) — do NOT copy JS bugs blindly
- **OmniRoute v3.8.50** (`~/dev/OmniRoute`, fallback `/tmp/omniroute_v3850` SHA `6cd4d38`) — authoritative reference for provider parity. When a provider behaves unexpectedly, compare against OmniRoute's implementations, not the legacy JS `9router`. Key areas: `open-sse/services/` + `src/lib/db/` + `open-sse/executors/` (credential health `HealthStatus`: 200/401/429/503/500, health checks, `discoverProviderModels` GET `/v1/models` with provider-specific headers, model-import merge/sync via `preserveRemovedCustomModelCompat`). Header-fidelity rules for parity: OpenRouter must send `HTTP-Referer` + `X-Title`; nvidia/llm7 omit `HTTP-Referer`; gemini sends `x-goog-api-key`; kiro/opencode/free-providers use the appropriate OAuth token flow. Always check `~/dev/OmniRoute` (fallback `/tmp/omniroute_v3850`) before making provider-specific decisions.

## Guiding Principle — Lightweight OmniRoute Clone

CipherRoute's end goal is a lightweight, stripped Rust clone of OmniRoute — **avoid OmniRoute's bloat**. When any feature's objective or implementation is unclear/ambiguous, the agent **must consult `~/dev/OmniRoute` (fallback `/tmp/omniroute_v3850`) first** rather than reinventing. No feature should be built from scratch without first checking OmniRoute — every future feature already exists there in some form.

**Lookup order:** `~/dev/OmniRoute/src/` + `~/dev/OmniRoute/open-sse/` → `/tmp/omniroute_v3850/src/managed/` (legacy).

**Explicitly out-of-scope / do NOT port (OmniRoute bloat to avoid):**
- MCP servers, A2A server, ACP protocol
- Electron/PWA/desktop, VNC
- Memory persistence, skills/conductor frameworks
- Gamification, analytics/telemetry/monitoring (beyond minimal usage stats)
- Cloud/sync/relay/storage backends
- Telegram bots, webhooks, third-party notifiers
- Chaos engineering, headroom, and any compression beyond RTK/Caveman (the only two kept).

## Dev Workflow — backend + dashboard rebuild

Single smooth loop — backend and dashboard are **separate builds** served by the same binary.
`./scripts/dev.sh` is repo-root-aware (`BASH_SOURCE`+`REPO_ROOT` `cd`) — always run as `./scripts/dev.sh` from repo root or any cwd, never `cd scripts`.

**Dashboard is not live-reloaded.** `web/src` → `web/dist` (Astro) is what the Rust server serves.
After any `web/src` change you **must** rebuild the dashboard or the feature will be invisible
(past "feature not found" confusion was a missing rebuild, not a missing backend).

### `scripts/dev.sh` presets — which to use when

| Change you made | Command (from repo root) | What it does | When to use |
|---|---|---|---|
| **Only `web/src`** (dashboard/providers, Astro, Tailwind) | `./scripts/dev.sh --web-only` or `./scripts/dev.sh --fast detach` | Rebuilds `web/dist` only if stale (`web/src` newer than `web/dist`), skips cargo | Iterating on UI; fastest feedback |
| **Only `src/`** (Rust: executors, translators, `src/db/`, `src/server/`) | `./scripts/dev.sh --backend-only detach` or `./scripts/dev.sh --fast detach` | `cargo build --bin cipherroute` (debug) + restart on `:4623` | Iterating on backend |
| **Both `web/src` + `src/`** | `./scripts/dev.sh --fast detach` | Stale-aware web + incremental cargo; `~10-20s` | Default daily loop |
| **Before `git push` / PR** | `./scripts/dev.sh --full detach` | Always rebuilds web + cargo + runs `cargo fmt --check`, `cargo clippy`, `astro check`, tests; `~2-5m` | Required gate before push — catches stale `web/dist` + lint failures |
| **Touched `provider_catalog.json` or `src/db/`** | `./scripts/dev.sh --full detach` | Same as above | Catalog/DB changes affect placeholder seeding (`GET /api/providers`) — needs full verification |
| **"Can't find providers" / stale `web/dist` suspect** | `./scripts/dev.sh --full detach` or `./scripts/dev.sh --web-only` | Forces `pnpm build` (`web/dist/_astro/*.js` hash changes, e.g. `D0OBBUDN.js` stale) | User reports blank `/dashboard/providers` or `ReferenceError: freeEntries` |
| **Lint without building** | `./scripts/dev.sh --check` | `fmt --check && clippy && astro check` (no build) | Pre-commit sanity |

Full flag reference:

```bash
./scripts/dev.sh --help
# --fast          Fast incremental build (default). Cargo debug + web only if stale. ~10-20s.
# --full          Full rebuild + checks. Always rebuilds web, runs fmt/clippy/tests. ~2-5m.
# --web-only      Only rebuild web/dist (pnpm build).
# --backend-only  Only rebuild Rust binary (cargo build).
# --release       Use release profile (implies slower optimized build).
# --port PORT     Server port (default 4623, also $PORT).
# --no-restart    Build only, don't start server (alias for MODE=build).
# MODE (legacy positional, still supported): run | detach | build | check | check-stale
```

Examples:

```bash
./scripts/dev.sh --fast detach        # edit web/src → quick rebuild, skip checks (daily)
./scripts/dev.sh --full detach        # before push: always web + full checks
./scripts/dev.sh --web-only           # touched only dashboard
./scripts/dev.sh --backend-only detach # touched only src/ Rust
./scripts/dev.sh --check              # lint without building
BUILD_MODE=release ./scripts/dev.sh --full  # optimized release build

# Legacy still works (maps to presets):
./scripts/dev.sh build                # → --fast --no-restart
./scripts/dev.sh detach               # → --fast detach
./scripts/dev.sh check                # → --check
```

**After ANY backend change, you MUST rebuild the binary and restart the server so the user can test it directly.** The running server does not hot-reload Rust. A common failure is leaving a stale binary running (an earlier build predating your fix) while reporting "done" — the user then tests the old behavior:

```bash
./scripts/dev.sh --fast detach        # fast path: incremental build + restart
curl -s http://127.0.0.1:4623/health  # confirm it is up
open http://127.0.0.1:4623/dashboard/providers
# or explicitly:
./scripts/dev.sh --backend-only detach && curl -s http://127.0.0.1:4623/health
```

If the change also touched `web/src`, the `--fast` preset already rebuilds `web/dist` when stale; use `--full` if you need to force it. Never report a fix as "done" or "ready to test" without completing this rebuild+restart. `scripts/dev.sh` (run/detach) warns on dirty `git status --porcelain` and on stale `web/dist`.

Raw web rebuild (without `dev.sh`) if needed:

```bash
cd web && pnpm install        # once
pnpm build                    # rebuild web/dist after every web/src change
pnpm dev                      # Astro dev on :4624 (proxy API to :4623) for iteration
```

## Contributing & Git Hygiene

Systematic, not arbitrary — all contributions follow two documents linked from the intelligence brief:

- **Workflow & expectations:** [`CONTRIBUTING.md`](CONTRIBUTING.md) — prerequisites, `scripts/dev.sh` quick/full, project layout, coding standards, testing matrix, beads parity workflow, secrets policy, releases.
- **Enforceable git rules:** [`docs/git-conventions.md`](docs/git-conventions.md) — branch naming (`<type>/<kebab>`), Conventional Commits (`<type>(<scope>): <subject>`), atomic bisectable commits, verification before each commit (`cargo fmt --check` + `cargo clippy --all-targets --all-features`), history hygiene (rebase, no `git add .`), PR hygiene (template, ≤400 lines, CI `web` → `rust` must be green), issue/beads discipline, tagging.

PRs use [`.github/pull_request_template.md`](.github/pull_request_template.md); bugs/features use [`.github/ISSUE_TEMPLATE/`](.github/ISSUE_TEMPLATE/). CI (`.github/workflows/ci.yml`) enforces `web: astro check + build → rust: fmt + clippy + tests` on `ubuntu` + `macos`. The checklist in `docs/git-conventions.md` §10 is the gate — all green means systematic.

## Agent Orchestration — Branch-per-Agent, Worktree Isolation

**Rule: one agent = one branch = one worktree. Never share a branch.**

- **2+ agents sharing same branch is disallowed.** Default is `branch-per-agent` (`<agent>/<type>/<kebab>` e.g. `a1/feat/provider-health`). If uncertain, revert to branch-per-agent. Shared-branch edits caused `git stash push -m "WIP: save changes from other agent's work"` (26-file stash `95111c5d`) and cross-branch cherry-picks on `feat/tokenizer-usage-compression-tracking`.
- **Worktree isolation (standard in agentic SWE):** each agent must run in its own `git worktree`:
  ```bash
  git worktree add ../wt-<agent>-<slug> -b <agent>/<type>/<kebab>
  cd ../wt-<agent>-<slug>
  # claim branch so others skip it
  ../cipherroute/scripts/claim-branch.sh <agent>/<type>/<kebab>
  ```
  Single checkout (`/home/atheerium/dev/cipherroute`) with N writers is the conflict root cause — `src/server/api/chat.rs` + `web/src/shared/constants/providers.ts` are hot files.
- **Claim file:** `scripts/claim-branch.sh` writes `.opencode/claims/<branch>` (gitignored) with `agent, pid, branch, timestamp`. Pre-task, check `cat .opencode/claims/*` or `git branch -a` for taken names.
- **Dirty-tree guard:** `scripts/dev.sh` (run/detach) now warns on dirty `git status --porcelain`; hooks block committing on wrong branch. Before switching, `git status --porcelain` must be clean.
- **Mechanical gates (not docs-only):** `scripts/setup-hooks.sh` installs `pre-commit` (fmt check + secret scan), `commit-msg` (Conventional Commits), `pre-push` (branch name + secret scan). `scripts/dev.sh check` runs `cargo fmt --check && cargo clippy --all-targets --all-features`. CI lints branch names.
- **Never stash to share:** stash is not a commit store; use `git worktree` + atomic commits. `STATE.md: NEVER commit` is superseded — agents **must** commit via `scripts/claim-branch.sh` branches and open PRs.
- See [`docs/agent-orchestration.md`](docs/agent-orchestration.md) for full worktree/claim/hook spec and recovery.

## Agent Orchestration — Branch-per-Agent, Worktree Isolation

**Rule: one agent = one branch = one worktree. Never share a branch.**

- **2+ agents sharing same branch is disallowed.** Default is `branch-per-agent` (`<agent>/<type>/<kebab>` e.g. `a1/feat/provider-health`). If uncertain, revert to branch-per-agent. Shared-branch edits caused `git stash push -m "WIP: save changes from other agent's work"` (26-file stash `95111c5d`) and cross-branch cherry-picks on `feat/tokenizer-usage-compression-tracking`.
- **Worktree isolation (standard in agentic SWE):** each agent must run in its own `git worktree`:
  ```bash
  git worktree add ../wt-<agent>-<slug> -b <agent>/<type>/<kebab>
  cd ../wt-<agent>-<slug>
  # claim branch so others skip it
  ../cipherroute/scripts/claim-branch.sh <agent>/<type>/<kebab>
  ```
  Single checkout (`/home/atheerium/dev/cipherroute`) with N writers is the conflict root cause — `src/server/api/chat.rs` + `web/src/shared/constants/providers.ts` are hot files.
- **Claim file:** `scripts/claim-branch.sh` writes `.opencode/claims/<branch>` (gitignored) with `agent, pid, branch, timestamp`. Pre-task, check `cat .opencode/claims/*` or `git branch -a` for taken names.
- **Dirty-tree guard:** `scripts/dev.sh` (run/detach) now warns on dirty `git status --porcelain`; hooks block committing on wrong branch. Before switching, `git status --porcelain` must be clean.
- **Mechanical gates (not docs-only):** `scripts/setup-hooks.sh` installs `pre-commit` (fmt check + secret scan), `commit-msg` (Conventional Commits), `pre-push` (branch name + secret scan). `scripts/dev.sh check` runs `cargo fmt --check && cargo clippy --all-targets --all-features`. CI lints branch names.
- **Never stash to share:** stash is not a commit store; use `git worktree` + atomic commits. `STATE.md: NEVER commit` is superseded — agents **must** commit via `scripts/claim-branch.sh` branches and open PRs.
- See [`docs/agent-orchestration.md`](docs/agent-orchestration.md) for full worktree/claim/hook spec and recovery.

## Core Product Surfaces (TOP PRIORITY)

These 4 surfaces ARE the product. Everything else is optional. They must be flawless, reliable, and mutually consistent — always prioritize regressions and improvements here:

1. **Providers page** — `/dashboard/providers/<provider>` (e.g. kilocode): user controls Available Models (disable/enable/custom). Configuration is user data, persisted in SQLite — must survive binary rebuilds/updates.
2. **CLI tools config** — `/dashboard/cli-tools/opencode` (opencode is the primary client).
3. **Combos page** — `/dashboard/combos`.
4. **`web/src/shared/components/ModelSelectModal.tsx`** — the single model-picker used everywhere; must exactly mirror the provider page's Available Models (same disabled map + custom rows + catalog merge). Any change to model-list logic MUST be applied consistently to both the provider page and this modal.

Core workflow that must never break: configure provider → customize available models → create combos → select models for opencode CLI config.

## Status
Active parity port. Run `cargo test -p cipherroute --lib parity_tests stream_flags` for smoke.

## Local Config & Secrets — Never Commit
- **Do not commit** local user config or secrets: `opencode.json`, `.env`, `.env.*`, `*.pem`, `~/.cipherroute/db.json`, `~/.cipherroute/admin.key`, API keys, `provider_specific_data` with live credentials, or any file containing `sk-`, `Bearer`, `refresh_token`.
- `opencode.json` is local agent config (model, MCP keys like `CONTEXT7_API_KEY`, permissions) — keep untracked. `scripts/dev.sh` builds locally; real secrets live in SQLite (`db.json` encrypted) + `CIPHERROUTE_API_KEY` env, not in git.
- Before `git add`/`commit`, run `git status` and `git diff --cached`; if a file contains secrets or is machine-local, `git restore --staged <file>` and add it to `.gitignore`. Prefer `git check-ignore -v <file>` to verify.
- If a secret is accidentally committed, rotate it immediately and purge history (`git filter-repo` or BFG) — do not just revert.

## Schema stability (`cipherroute.v1.*`)

The `cipherroute.v1.*` envelope namespace is a **frozen, additive-only contract**. Every JSON envelope emitted by `--robot` carries a `schema` field matching `cipherroute.v1.<area>.<action>`. Existing fields keep their names, types, and meanings across releases. New fields are additive only — no renames or removals. A new `cipherroute.v2.*` namespace will be opened before any breaking change.

Run `cipherroute schema stability` to see the current stability promise:

```bash
cipherroute --robot schema stability
# → {"schema":"cipherroute.v1.schema.stability","data":{"namespace":"cipherroute.v1","stability":"stable","policy":"..."}}
```

The `schema` subcommand provides four operations:

| Command | Purpose |
|---|---|
| `cipherroute schema list` | List all resource kinds with schema and example support |
| `cipherroute schema show <resource>` | Print JSON Schema for a resource (provider, key, combo, etc.) |
| `cipherroute schema example <resource>` | Print an example payload for a resource |
| `cipherroute schema stability` | Print the v1 namespace stability contract |

13 resources are covered: `provider`, `provider-node`, `combo`, `key`, `pool`, `settings`, `custom-model`, `model-alias`, `usage-event`, `log-event`, `chat-event`, `quota`, `oauth-status`. Each has both a schema and an example — enforced by tests.

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

## Agent Orchestration — Branch-per-Agent, Worktree Isolation

**Rule: one agent = one branch = one worktree. Never share a branch.**

- **2+ agents sharing same branch is disallowed.** Default is `branch-per-agent` (`<agent>/<type>/<kebab>` e.g. `a1/feat/provider-health`). If uncertain, revert to branch-per-agent. Shared-branch edits caused `git stash push -m "WIP: save changes from other agent's work"` (26-file stash `95111c5d`) and cross-branch cherry-picks on `feat/tokenizer-usage-compression-tracking`.
- **Worktree isolation (standard in agentic SWE):** each agent must run in its own `git worktree`:
  ```bash
  git worktree add ../wt-<agent>-<slug> -b <agent>/<type>/<kebab>
  cd ../wt-<agent>-<slug>
  # claim branch so others skip it
  ../openproxy/scripts/claim-branch.sh <agent>/<type>/<kebab>
  ```
  Single checkout (`/home/atheerium/dev/openproxy`) with N writers is the conflict root cause — `src/server/api/chat.rs` + `web/src/shared/constants/providers.ts` are hot files.
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
Active parity port. Run `cargo test -p openproxy --lib parity_tests stream_flags` for smoke.

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

# Agent Orchestration — Worktree + Branch-per-Agent + Mechanical Gates

## Problem

Single checkout `/home/atheerium/dev/openproxy` with N concurrent agents writing the same hot files (`src/server/api/chat.rs`, `web/src/shared/constants/providers.ts`, `provider_catalog.json`) causes silent overwrites. Symptoms observed:

- `git stash push -m "WIP: save changes from other agent's work"` with 26-file stash `95111c5d`
- Branch `feat/tokenizer-usage-compression-tracking` used as shared inbox for analytics+oauth+UI (16 commits, 6 concerns → violates atomic PR)
- 3 live stashes as shadow commit store, reflog `reset`/`rebase --abort` cycles
- `web/dist` stale because agents edited `web/src` without `pnpm build` + Rust rebuild

## Rule

**One agent = one branch = one worktree. Never share a branch.**

If uncertain, revert to `branch-per-agent`. `2+ agents on same branch is disallowed`.

## Quick Start (every agent, every task)

```bash
# 0. check what's already claimed
cat .opencode/claims/* 2>/dev/null; git branch -a | head -n 30; git worktree list

# 1. create isolated worktree + branch (name includes agent id)
git worktree add ../wt-a1-kilo-health -b a1/feat/kilo-health
cd ../wt-a1-kilo-health

# 2. claim branch (writes .opencode/claims/<branch> gitignored)
../openproxy/scripts/claim-branch.sh a1/feat/kilo-health

# 3. work, commit atomically, push, open PR
git status; git diff --cached   # never git add . blindly
cargo fmt --all && cargo clippy --all-targets --all-features
./scripts/dev.sh build && (cd web && pnpm build)  # if web/src changed
git commit -m "feat(providers): ..."
git push -u origin a1/feat/kilo-health
gh pr create --fill

# 4. on done, remove worktree (keep branch until PR merged)
git worktree remove ../wt-a1-kilo-health
```

## Naming

```
<agent>/<type>/<kebab>
# agent: a1, a2, jcode-xyz, etc. — short, unique
# type:  feat|fix|chore|refactor|test|docs|ci|build|perf  (mirrors git-conventions.md §1)
```

Examples: `a1/feat/provider-health`, `a2/fix/providers-401-banner`, `b3/chore/setup-hooks`

CI lints branch names (`.github/workflows/ci.yml` → `lint-branch-name` job). Non-conforming branches fail fast.

## Claim File

`scripts/claim-branch.sh <branch>` writes `.opencode/claims/<branch>` containing `agent, pid, branch, timestamp`. File is gitignored. Other agents check claims before picking a name. Claims are advisory + audit trail; `git branch -a` is the source of truth.

Manual claim inspection:

```bash
ls .opencode/claims/
cat .opencode/claims/a1__feat__kilo-health  # branch / in filename is __
```

On worktree removal or branch deletion, claim is cleaned (`git worktree remove` prunes; `scripts/claim-branch.sh --release <branch>` also works).

## Worktree Isolation Details

- Single checkout with N writers is the root cause. `git worktree` gives each agent a full checkout sharing the same `.git` object store — no extra clone, instant.
- Do **not** `cd` back to the original checkout to commit. Commit inside the worktree so `HEAD` is unambiguous.
- Hot files: `src/server/api/chat.rs`, `src/core/executor/*.rs`, `web/src/components/providers/ProviderDetailPageClient.tsx`, `web/src/shared/constants/providers.ts`, `src/core/config/provider_catalog.json`, `web/src/shared/components/ModelSelectModal.tsx` — partition work by file ownership where possible.

## Dirty-Tree Guard

`scripts/dev.sh` (run/detach) warns on dirty `git status --porcelain`. Hooks block `git commit` on wrong branch. Standard flow:

```bash
git status --porcelain   # must be clean before switching worktrees
./scripts/dev.sh check   # fmt + clippy gate before commit
```

## Mechanical Gates (not docs-only)

| Gate | Location | What it does |
|------|----------|--------------|
| `setup-hooks.sh` | `scripts/setup-hooks.sh` | Installs executable hooks from `.githooks/` into `.git/hooks/` |
| `pre-commit` | `.githooks/pre-commit` | `cargo fmt --check`, secret scan (`sk-...`, `refresh-token`, `opencode.json`), block `git add .` without review |
| `commit-msg` | `.githooks/commit-msg` | Conventional Commits regex (`^(feat|fix|...)(\(.+\))?: .+`) + length checks |
| `pre-push` | `.githooks/pre-push` | Branch name lint + secret scan + warn if `web/src` newer than `web/dist` |
| `dev.sh check` | `scripts/dev.sh check` | Local `fmt --check && clippy --all-targets --all-features && astro check` |
| CI `lint-branch-name` | `.github/workflows/ci.yml` | Fails PR if branch name violates `<type>/<kebab>` or `<agent>/<type>/<kebab>` |

Enable hooks once per clone:

```bash
./scripts/setup-hooks.sh
```

Hooks are **not** bypassed with `--no-verify` without immediate re-check.

## Why Not Stash

`git stash` is not a commit store. `git stash list` is invisible to `git log` and `git branch -a`. Work lost in stash requires `git stash branch temp/rescue stash@{0}` to recover and guarantees merge conflicts with later `cargo fmt` commits. Use worktree + atomic commits.

## Recovery

- `git stash push -m "WIP: ..."`: convert to branch immediately `git stash branch temp/rescue stash@{0}`
- `git worktree list` shows stale entries: `git worktree prune` + `rm -rf ../wt-...`
- Branch collision: `git branch -a` shows taken names; pick `<agent>/<type>/<kebab>` with different `agent` prefix
- Worktree claims stale: `ls .opencode/claims/` + check `git branch -a` — delete claim if branch gone

## References

- `AGENTS.md` § Agent Orchestration — user-facing summary
- `docs/git-conventions.md` — branch naming, Conventional Commits, atomic commits
- `scripts/claim-branch.sh` — claim implementation
- `scripts/setup-hooks.sh` + `.githooks/*` — hook implementations
- `.opencode/claims/` — runtime claim storage (gitignored)

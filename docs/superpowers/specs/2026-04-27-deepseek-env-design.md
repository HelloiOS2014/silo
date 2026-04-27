# DeepSeek silo Environment — Design

**Status:** approved (brainstorming → spec)
**Author:** Claude Code session, 2026-04-27
**Scope:** Add a new silo environment `deepseek` for running Claude Code against DeepSeek's Anthropic-compatible API endpoint.

## Goal

Provide a one-command path to launch Claude Code against DeepSeek (`silo exec -e deepseek -- claude`) with full credential / config / history isolation from any other silo environment and from the host.

This is a configuration-only change. No silo source code is modified.

## Background

silo already supports the "Claude Code + Anthropic-compatible third-party endpoint" pattern, documented in `docs/guide-claude-code-providers.md` and exemplified by the `minimax` and `kimi` environments. DeepSeek published an Anthropic-compatible endpoint at `https://api.deepseek.com/anthropic` (verified against `api-docs.deepseek.com/guides/anthropic_api` and `/guides/coding_agents`), so the same pattern applies with three substantive differences from minimax:

1. Different base URL, key variable name, and model identifiers.
2. DeepSeek's official Claude Code guide recommends two extra Claude-Code-specific knobs (`CLAUDE_CODE_SUBAGENT_MODEL`, `CLAUDE_CODE_EFFORT_LEVEL`) that minimax does not set.
3. Onboarding (`mkdir ~/.claude` + write `.claude.json` marker) is moved into a `[setup].on_init` hook so first-run is `silo setup -e deepseek` instead of two manual `silo exec` lines.

## Non-Goals

- Not running raw `deepseek` SDK or any non-Claude-Code CLI.
- Not bridging through a self-hosted proxy (`claude-code-proxy` etc.). Official endpoint only.
- Not copying any host `~/.claude/` content (skills, CLAUDE.md, settings) into the isolated HOME. Clean start.
- Not adding macOS Keychain support for this env. Stays on the project's existing envfile pattern.
- Not installing Claude Code inside the env via `npm i -g @anthropic-ai/claude-code`. The env relies on the host-installed `claude` exposed through `PATH`.

## Architecture

```
host shell
  └─ silo exec -e deepseek -- claude
       └─ silo builds clean child env:
            HOME                  = ~/.silo/deepseek/home
            XDG_* / TMPDIR        = ~/.silo/deepseek/{config,cache,data,state,tmp}
            ANTHROPIC_BASE_URL    = https://api.deepseek.com/anthropic
            ANTHROPIC_AUTH_TOKEN  = <from secrets.env>
            ANTHROPIC_MODEL       = deepseek-v4-pro
            ANTHROPIC_DEFAULT_OPUS_MODEL    = deepseek-v4-pro
            ANTHROPIC_DEFAULT_SONNET_MODEL  = deepseek-v4-pro
            ANTHROPIC_DEFAULT_HAIKU_MODEL   = deepseek-v4-flash
            CLAUDE_CODE_SUBAGENT_MODEL      = deepseek-v4-flash
            CLAUDE_CODE_EFFORT_LEVEL        = max
            CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = 1
            API_TIMEOUT_MS        = 3000000
            (forced) HOME / XDG_* / TMPDIR / SILO_ROOT / SILO_HOST_HOME
       └─ exec npm-installed `claude`
            └─ requests hit api.deepseek.com/anthropic
            └─ creds and conversation history land only in isolated HOME
```

### Invariants

- Host `ANTHROPIC_*` and `OPENAI_API_KEY` are explicitly denied to prevent leakage even if a future maintainer adds them to `[env].allow`.
- `secrets.env` is mode 600; silo enforces this at read time.
- The setup hook is idempotent (gated by `.setup-done`); rerun via `silo setup -e deepseek --force`.
- No host `~/.claude/` content is copied — clean start.

## Manifest

`~/.silo/deepseek/manifest.toml`:

```toml
id = "deepseek"
root = "/Users/panghu/.silo/deepseek"

[env]
allow = ["PATH", "TERM", "LANG", "LC_ALL", "COLORTERM"]
deny = [
  "ANTHROPIC_API_KEY",
  "ANTHROPIC_AUTH_TOKEN",
  "ANTHROPIC_BASE_URL",
  "ANTHROPIC_MODEL",
  "OPENAI_API_KEY",
]

[env.set]
ANTHROPIC_BASE_URL                       = "https://api.deepseek.com/anthropic"
ANTHROPIC_MODEL                          = "deepseek-v4-pro"
ANTHROPIC_DEFAULT_OPUS_MODEL             = "deepseek-v4-pro"
ANTHROPIC_DEFAULT_SONNET_MODEL           = "deepseek-v4-pro"
ANTHROPIC_DEFAULT_HAIKU_MODEL            = "deepseek-v4-flash"
CLAUDE_CODE_SUBAGENT_MODEL               = "deepseek-v4-flash"
CLAUDE_CODE_EFFORT_LEVEL                 = "max"
API_TIMEOUT_MS                           = "3000000"
CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = "1"
AI_ENV                                   = "deepseek"

[secrets]
provider = "envfile"
items = ["ANTHROPIC_AUTH_TOKEN"]

[network]
mode = "default"

[setup]
on_init = [
  "mkdir -p $HOME/.claude",
  "echo '{\"hasCompletedOnboarding\": true}' > $HOME/.claude.json",
]
```

### Field Rationale

- **`deny` includes `ANTHROPIC_MODEL`** — redundant with the clean-env default but prevents surprise if `[env].allow` is later widened.
- **`API_TIMEOUT_MS = 3000000`** (50 min) — DeepSeek "thinking mode" responses can be slow; matches the minimax precedent.
- **`CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = 1`** — telemetry would otherwise hit Anthropic infrastructure with metadata about requests that actually went to DeepSeek.
- **`secrets.items` has only `ANTHROPIC_AUTH_TOKEN`** — DeepSeek's Anthropic endpoint requires only this one secret. No analog to minimax's `MINIMAX_API_KEY`.
- **No `env.prepend`** — `claude` is on host PATH; no per-env PATH augmentation needed.

## Secrets

`~/.silo/deepseek/secrets.env`, mode 600:

```
ANTHROPIC_AUTH_TOKEN=sk-<deepseek-api-key>
```

Source the key from <https://platform.deepseek.com/api_keys>.

## Setup Flow

```bash
silo init -e deepseek                          # 1. scaffold env directory + default manifest
$EDITOR ~/.silo/deepseek/manifest.toml         # 2. replace contents with the manifest above
cat > ~/.silo/deepseek/secrets.env <<'EOF'     # 3. write key
ANTHROPIC_AUTH_TOKEN=sk-<deepseek-api-key>
EOF
chmod 600 ~/.silo/deepseek/secrets.env
silo setup -e deepseek                         # 4. runs on_init: mkdir + .claude.json marker
silo show -e deepseek                          # 5. inspect resolved config
silo exec -e deepseek -- claude                # 6. launch
```

## Verification

Manual smoke-test tiers; cargo tests are unaffected (no source change, current 64 tests must remain green).

| Tier | Command | Pass criteria | Token cost |
|------|---------|---------------|-----------|
| 1. Config parses | `silo show -e deepseek` | Resolved env shows base URL / model / envfile provider / default network | none |
| 2. Process isolation | `silo exec -e deepseek -- env \| sort` | `HOME` / `XDG_*` redirected; no host `*_API_KEY` leakage; `ANTHROPIC_AUTH_TOKEN` present; `SILO_HOST_HOME` set | none |
| 3. Onboarding landed | `cat ~/.silo/deepseek/home/.claude.json` and `ls ~/.silo/deepseek/.setup-done` | `.claude.json` is `{"hasCompletedOnboarding": true}`; `.setup-done` exists | none |
| 4. End-to-end | `silo exec -e deepseek -- claude` then `/status` and "hi" | `/status` shows `deepseek-v4-pro` and base URL; "hi" returns a response | one tiny call |

### Failure Modes

- 401 / 403 → wrong or revoked key. Edit `secrets.env`; do NOT need `silo setup --force`.
- "model not found" → check exact model id (`deepseek-v4-pro` / `deepseek-v4-flash`).
- Connection timeout → upstream DeepSeek issue, unrelated to silo.
- Onboarding prompt reappears → `.claude.json` missing or wrong; rerun `silo setup -e deepseek --force`.

## Risks / Open Items

- **Host `claude` install path.** If the host installed Claude Code via the native installer (`~/.local/bin/claude`), `~/.local/bin` must be on PATH inside the env. silo inherits `PATH` from `[env].allow`, so as long as the host PATH already includes `~/.local/bin` (per project install convention) this works without changes. Documented as a footnote in the README provider guide.
- **Model name churn.** DeepSeek has been bumping versions (V3.1 → V3.2 → V4). The manifest pins `deepseek-v4-pro` / `deepseek-v4-flash` — accurate as of 2026-04-27. Any future model rename is a one-line manifest edit.
- **`ANTHROPIC_AUTH_TOKEN` value in env dump.** Tier-2 verification command (`silo exec -e deepseek -- env`) prints the token to stdout. This is local-only and acceptable for one-time validation, but should not be scripted into automated tests or committed logs.

## Out-of-Scope Future Work

- Add a `docs/guide-claude-code-providers.md` section for DeepSeek mirroring the minimax/kimi sections.
- A `Setting up DeepSeek` walkthrough in `README.md` if DeepSeek becomes a primary supported provider.
- Migrating any of the existing envs to keychain provider (separate decision; out of scope here).

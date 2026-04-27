# DeepSeek silo Environment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a working `deepseek` silo environment that lets `silo exec -e deepseek -- claude` run Claude Code against DeepSeek's Anthropic-compatible endpoint, fully isolated from host and other envs.

**Architecture:** Configuration-only change — no Rust source modified. We scaffold a new env directory under `~/.silo/deepseek/`, replace the default manifest with a DeepSeek-tuned one, drop a mode-600 `secrets.env` containing `ANTHROPIC_AUTH_TOKEN`, run the setup hook to create the Claude Code onboarding marker, and validate through a tiered smoke test (config parse → env isolation → onboarding → end-to-end).

**Tech Stack:** silo (Rust CLI, already built), Claude Code (npm-installed `claude` on host PATH), DeepSeek Anthropic-compatible endpoint at `https://api.deepseek.com/anthropic`.

**Spec reference:** `docs/superpowers/specs/2026-04-27-deepseek-env-design.md`

---

## File Structure

This plan creates exactly three files, all under `~/.silo/deepseek/` (outside the git repo):

| Path | Created by | Purpose |
|------|------------|---------|
| `~/.silo/deepseek/manifest.toml` | `silo init` then overwritten by Task 2 | Environment configuration |
| `~/.silo/deepseek/secrets.env` | Task 3 (manual `cat`) | Holds `ANTHROPIC_AUTH_TOKEN`, mode 600 |
| `~/.silo/deepseek/home/.claude.json` | Task 4 setup hook | Claude Code onboarding marker |

Also auto-created by `silo init`: `home/`, `config/`, `cache/`, `data/`, `state/`, `tmp/`, `run/`, plus a default `env.zsh` (we leave it untouched).

After the setup hook runs successfully, a `.setup-done` marker appears in `~/.silo/deepseek/`.

**No files in this git repository are modified.** This plan does not touch Rust source, tests, README, or docs (those are out-of-scope future work per spec).

---

## Prerequisites Check (one-time, do before Task 1)

- [ ] **Step 0a: Confirm `silo` is on PATH and built from current source**

Run: `which silo && silo --version`
Expected: a path under `~/.local/bin/silo` (per CLAUDE.md install convention) and the current crate version.

If not installed: `cd /Users/panghu/code/rsearch/llm_env && cargo install --path . --root ~/.local --force`

- [ ] **Step 0b: Confirm host `claude` is reachable**

Run: `which claude && claude --version`
Expected: a path (typically under the npm global prefix, e.g. `/usr/local/bin/claude` or `/opt/homebrew/bin/claude`) and a version string.

If missing: `npm install -g @anthropic-ai/claude-code`. (Native installer at `~/.local/bin/claude` also works since `~/.local/bin` is on PATH.)

- [ ] **Step 0c: Confirm a `deepseek` environment does not already exist**

Run: `silo ls`
Expected: output does not contain `deepseek`. If it does, decide whether to delete (`rm -rf ~/.silo/deepseek`) or pick a different name — STOP and ask the user.

- [ ] **Step 0d: Have the DeepSeek API key ready**

You need a key from <https://platform.deepseek.com/api_keys>. Format: `sk-...`. Keep it in a paste buffer — you'll inject it in Task 3. Do **not** commit it anywhere.

---

## Task 1: Scaffold the environment directory

**Files:**
- Create: `~/.silo/deepseek/` (entire tree, via `silo init`)

- [ ] **Step 1.1: Run `silo init`**

Run: `silo init -e deepseek`

Expected stdout (something like):
```
created environment "deepseek" at /Users/panghu/.silo/deepseek
```

- [ ] **Step 1.2: Verify the directory layout**

Run: `ls -la ~/.silo/deepseek/`

Expected: presence of `manifest.toml`, `env.zsh`, and the directories `home/`, `config/`, `cache/`, `data/`, `state/`, `tmp/`. (No `run/` yet — created on first exec.)

- [ ] **Step 1.3: Verify `silo ls` lists the new env**

Run: `silo ls`

Expected: output includes `deepseek` alongside `minimax` and `xiaomi`.

(No commit — files live outside the repo.)

---

## Task 2: Write the DeepSeek-tuned manifest

**Files:**
- Modify: `~/.silo/deepseek/manifest.toml` (overwrite the default scaffolded by Task 1)

- [ ] **Step 2.1: Overwrite manifest with the final content**

Write this exact file to `~/.silo/deepseek/manifest.toml`:

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

[env.prepend]
PATH = "$HOME/.local/bin"

[secrets]
provider = "envfile"
items = ["ANTHROPIC_AUTH_TOKEN"]

[network]
mode = "default"

[setup]
on_init = [
  "mkdir -p $HOME/.claude",
  "echo '{\"hasCompletedOnboarding\": true}' > $HOME/.claude.json",
  "mkdir -p $HOME/.local/bin",
  "ln -sf $SILO_HOST_HOME/.local/bin/claude $HOME/.local/bin/claude",
]
```

- [ ] **Step 2.2: Validate the manifest by parsing it through silo**

Run: `silo show -e deepseek`

Expected: command exits 0 and prints a resolved configuration that contains the lines (order may vary):
- `id: deepseek`
- `root: /Users/panghu/.silo/deepseek`
- `ANTHROPIC_BASE_URL = https://api.deepseek.com/anthropic`
- `ANTHROPIC_MODEL = deepseek-v4-pro`
- `secrets.provider = envfile`
- `network.mode = default`

If `silo show` errors with a TOML parse error or `manifest validation` failure, re-check the file character-by-character against the block above (common issues: stray smart quotes, missing commas in `deny =`, mistyped reserved key in `env.set`).

(No commit.)

---

## Task 3: Provision the secret

**Files:**
- Create: `~/.silo/deepseek/secrets.env`

- [ ] **Step 3.1: Write the secrets file**

Run (replace `sk-REPLACE_ME` with the actual key from Step 0d):

```bash
cat > ~/.silo/deepseek/secrets.env <<'EOF'
ANTHROPIC_AUTH_TOKEN=sk-REPLACE_ME
EOF
chmod 600 ~/.silo/deepseek/secrets.env
```

- [ ] **Step 3.2: Verify file mode is 600**

Run: `stat -f '%A %N' ~/.silo/deepseek/secrets.env`

Expected: starts with `600 ` followed by the path. If silo is run against a more permissive file (e.g. 644), it will refuse to read secrets.

- [ ] **Step 3.3: Verify silo can resolve the secret**

Run: `silo exec -e deepseek -- sh -c 'echo "TOKEN_LEN=${#ANTHROPIC_AUTH_TOKEN}"'`

Expected: prints something like `TOKEN_LEN=51` (a non-zero positive integer matching DeepSeek key length). If it prints `TOKEN_LEN=0`, the envfile didn't load — re-check the file format (no spaces around `=`, no surrounding quotes unless intended).

This deliberately does NOT print the token itself.

(No commit.)

---

## Task 4: Run the setup hook

**Files:**
- Create: `~/.silo/deepseek/home/.claude/` (directory)
- Create: `~/.silo/deepseek/home/.claude.json`
- Create: `~/.silo/deepseek/.setup-done`

- [ ] **Step 4.1: Run `silo setup`**

Run: `silo setup -e deepseek`

Expected stdout:
```
[setup 1/2] mkdir -p $HOME/.claude
[setup 2/2] echo '{"hasCompletedOnboarding": true}' > $HOME/.claude.json
setup complete (2 commands)
```

- [ ] **Step 4.2: Verify `.claude` directory exists in isolated HOME**

Run: `ls -la ~/.silo/deepseek/home/.claude/`

Expected: directory exists (empty is fine).

- [ ] **Step 4.3: Verify the onboarding marker content**

Run: `cat ~/.silo/deepseek/home/.claude.json`

Expected exact output:
```
{"hasCompletedOnboarding": true}
```

- [ ] **Step 4.4: Verify `.setup-done` marker**

Run: `ls ~/.silo/deepseek/.setup-done`

Expected: file exists. (Re-running `silo setup -e deepseek` should now print `setup already completed (use --force to re-run)` and not re-execute the hooks.)

- [ ] **Step 4.5: Confirm idempotency**

Run: `silo setup -e deepseek`

Expected: prints `setup already completed (use --force to re-run)` and exits 0 without running the hooks.

(No commit.)

---

## Task 5: Verify environment isolation (Tier 2 of spec smoke test)

**Files:** none modified.

- [ ] **Step 5.1: Dump the resolved child environment**

Run: `silo exec -e deepseek -- sh -c 'env | sort' | grep -E '^(HOME|XDG_|TMPDIR|ANTHROPIC_|CLAUDE_CODE_|SILO_|API_TIMEOUT_MS|AI_ENV|OPENAI_)'`

Expected lines (token redacted in your eyes — do **not** paste this output anywhere):
```
AI_ENV=deepseek
ANTHROPIC_AUTH_TOKEN=sk-...           ← present
ANTHROPIC_BASE_URL=https://api.deepseek.com/anthropic
ANTHROPIC_DEFAULT_HAIKU_MODEL=deepseek-v4-flash
ANTHROPIC_DEFAULT_OPUS_MODEL=deepseek-v4-pro
ANTHROPIC_DEFAULT_SONNET_MODEL=deepseek-v4-pro
ANTHROPIC_MODEL=deepseek-v4-pro
API_TIMEOUT_MS=3000000
CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1
CLAUDE_CODE_EFFORT_LEVEL=max
CLAUDE_CODE_SUBAGENT_MODEL=deepseek-v4-flash
HOME=/Users/panghu/.silo/deepseek/home
SILO_EXEC_DIR=/Users/panghu/.silo/deepseek/run/<pid>
SILO_HOST_HOME=/Users/panghu
SILO_ROOT=/Users/panghu/.silo
TMPDIR=/Users/panghu/.silo/deepseek/tmp
XDG_CACHE_HOME=/Users/panghu/.silo/deepseek/cache
XDG_CONFIG_HOME=/Users/panghu/.silo/deepseek/config
XDG_DATA_HOME=/Users/panghu/.silo/deepseek/data
XDG_STATE_HOME=/Users/panghu/.silo/deepseek/state
```

- [ ] **Step 5.2: Verify no host `OPENAI_API_KEY` leaked**

Run: `silo exec -e deepseek -- sh -c 'echo "OPENAI=${OPENAI_API_KEY:-<unset>}"'`

Expected: `OPENAI=<unset>` regardless of whether your host has `OPENAI_API_KEY` exported.

- [ ] **Step 5.3: Verify `SILO_HOST_HOME` points to the real host home**

Run: `silo exec -e deepseek -- sh -c 'echo $SILO_HOST_HOME'` and compare to `echo $HOME` from your host shell.

Expected: same value (the real host home, e.g. `/Users/panghu`).

(No commit.)

---

## Task 6: End-to-end validation (Tier 4 of spec smoke test)

This task spends ~1–2 minimal API calls.

- [ ] **Step 6.1: Launch Claude Code in the env**

Run: `silo exec -e deepseek -- claude`

Expected: Claude Code REPL starts directly (no onboarding wizard, no "agree to ToS" prompt). Cursor lands at the input prompt.

If onboarding wizard appears: Task 4 didn't write `.claude.json` correctly. Quit (Ctrl-C twice), inspect `~/.silo/deepseek/home/.claude.json`, fix, then `silo setup -e deepseek --force`, then retry.

- [ ] **Step 6.2: Run `/status` inside the REPL**

Type: `/status` then Enter.

Expected: status panel shows
- Model: `deepseek-v4-pro`
- API base URL: `https://api.deepseek.com/anthropic` (or a string clearly indicating DeepSeek)

If the status shows an Anthropic model (e.g. `claude-sonnet-...`) or `api.anthropic.com`, the `[env.set]` block didn't take effect. Quit, run `silo show -e deepseek` to check, and re-do Task 2 if the manifest is wrong.

- [ ] **Step 6.3: Send a trivial prompt**

Type: `hi` then Enter.

Expected: a short text response (1–3 sentences). This proves the auth token works and routing is correct.

Failure modes:
- 401 / 403 → bad token. Quit, re-check `~/.silo/deepseek/secrets.env`, no `silo setup --force` needed (onboarding marker is independent of the token).
- "model not found" → typo in `ANTHROPIC_MODEL`. Edit manifest, retry.
- Hang / timeout → DeepSeek-side issue. Try `curl -s https://api.deepseek.com/anthropic/v1/models -H "x-api-key: $YOUR_KEY"` from a host shell to isolate.

- [ ] **Step 6.4: Quit cleanly**

Type: `/exit` (or Ctrl-D).

Expected: returns to host shell.

(No commit.)

---

## Task 7: Final sanity sweep

**Files:** none modified.

- [ ] **Step 7.1: Confirm no stale state in host `~/.claude/`**

Run: `ls -la ~/.claude/projects/ 2>/dev/null | grep -i deepseek || echo "OK: no host pollution"`

Expected: `OK: no host pollution`. (Claude Code stores conversation history in `$XDG_CONFIG_HOME/claude` or `$HOME/.claude/projects/`; in the silo env both redirect into `~/.silo/deepseek/`. If you see deepseek-related entries in your host `~/.claude/`, isolation is broken — STOP and report.)

- [ ] **Step 7.2: Confirm conversation history landed in the isolated HOME**

Run: `find ~/.silo/deepseek/home/.claude -type f 2>/dev/null | head`

Expected: at least one file exists (the projects/history Claude Code wrote during Task 6.3). Empty output means Claude Code wrote elsewhere — investigate.

- [ ] **Step 7.3: Confirm cargo tests still pass**

Run: `cd /Users/panghu/code/rsearch/llm_env && cargo test --quiet`

Expected: all 64 tests pass. (No source changed, so this is a sanity check that the workspace is clean.)

- [ ] **Step 7.4: Confirm git working tree is unchanged by this work**

Run: `git -C /Users/panghu/code/rsearch/llm_env status --short`

Expected: only the pre-existing modifications shown at session start (`M .claude/settings.local.json` and untracked `.playwright-mcp/`), plus the spec/plan files in `docs/superpowers/`. No accidental edits to source.

(No commit — implementation is config-only and lives outside the repo. The spec was already committed in `7a7666d`; this plan will be committed as part of the writing-plans handoff below.)

---

## Done Criteria

All boxes above checked. The user can now run `silo exec -e deepseek -- claude` and chat with DeepSeek through Claude Code, with credentials and history isolated under `~/.silo/deepseek/`.

## Rollback

If anything goes wrong and you want a clean slate:

```bash
rm -rf ~/.silo/deepseek
```

This removes the entire env (manifest, secrets, isolated HOME, setup marker). No silo command, system config, or git state is affected.

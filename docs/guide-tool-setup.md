# Tool Installation Guide for Silo Environments

This guide explains how to install and configure various types of CLI tools inside silo isolated environments.

## How Silo Isolates Tools

| Mechanism | Effect |
|-----------|--------|
| `PATH` passthrough | Host binaries accessible without per-env installation |
| `HOME` redirection | `$HOME` → `{env_root}/home` |
| `XDG_*` redirection | `$XDG_CONFIG_HOME` → `{env_root}/config`, etc. |
| `SILO_HOST_HOME` | Points to the **real** host HOME (for copying files from host) |
| `[setup].on_init` | Declarative per-env initialization commands |

When you run `silo exec -e myenv -- some-tool`, the tool sees a completely different HOME and config directory. XDG-compliant tools automatically write their config to the isolated location.

## Quick Start

```bash
# 1. Install the tool on the host (once)
# 2. Create a silo environment
silo init -e myenv

# 3. Edit manifest to add setup hooks
#    ~/.silo/myenv/manifest.toml → add [setup] section

# 4. Run setup
silo setup -e myenv

# 5. Use it
silo exec -e myenv -- some-tool ...
```

---

## Patterns

### Pattern 1: Static Binary CLI (Go/Rust)

**Examples**: ios-pilot, rg, fd, jq, gh

Binary lives on the host PATH. Config is XDG-compliant, so it's automatically isolated.

```toml
[setup]
on_init = [
  "tool config init",
]
```

No per-env binary installation needed. The tool is already accessible via PATH.

### Pattern 2: CLI with Daemon/Socket

**Examples**: ios-pilot (daemon + Unix socket), Docker

Binary is shared. The daemon socket path follows `$XDG_CONFIG_HOME` or `$TMPDIR`, so each silo environment gets its own daemon instance.

```toml
[setup]
on_init = [
  "ios-pilot config init",
  "ios-pilot wda setup",
]
```

**Note**: If a tool hardcodes `~/.config/`, the socket still lands in the isolated location because `$HOME` is redirected. Verify by checking the tool's socket path resolution.

### Pattern 3: npm Global Packages

**Examples**: lark-cli (`@larksuite/cli`), typescript, eslint

npm global install writes to `$HOME`-relative paths. Since HOME is redirected, each silo environment gets its own npm global prefix and package tree.

```toml
[env]
allow = ["PATH"]

[env.set]
AI_ENV = "my-lark-env"
# Ensure npm global bin is in PATH
NPM_CONFIG_PREFIX = "$HOME/.npm-global"

[setup]
on_init = [
  "npm config set prefix $HOME/.npm-global",
  "npm install -g @larksuite/cli",
  "lark-cli config init",
]
```

**Key points**:
- `npm install -g` installs to the isolated HOME
- You may need to configure `NPM_CONFIG_PREFIX` and add its `bin/` to PATH
- Each environment maintains its own package versions independently

### Pattern 4: Tools with OAuth Authentication

**Examples**: lark-cli (`lark-cli auth login`), gh (GitHub CLI)

OAuth login creates per-env sessions. Auth tokens are stored in the config directory (isolated by XDG redirection).

```toml
[setup]
on_init = [
  "npm install -g @larksuite/cli",
  "lark-cli config init",
]
```

After setup, login interactively:

```bash
silo shell -e my-lark-env
lark-cli auth login
# Complete OAuth flow in browser
exit
```

**Keychain caveat**: macOS Keychain is system-wide, not per-environment. If a tool stores tokens in Keychain (not in config files), different silo environments may share auth state. Solutions:
- Check if the tool supports file-based token storage
- Use different Keychain service names (if the tool distinguishes by app ID)
- For lark-cli: configure different Feishu apps per environment

**Rule of thumb**: Interactive auth (browser-based OAuth) should be done via `silo shell`, not in `on_init`.

### Pattern 5: Tools with Skill/Plugin Ecosystems

**Examples**: lark-cli (skills), Claude Code (MCP plugins)

Beyond the main binary, these tools need additional skill or plugin packages.

```toml
[setup]
on_init = [
  "npm install -g @larksuite/cli",
  "npx skills add larksuite/cli -y -g",
  "lark-cli config init",
]
```

Each silo environment maintains its own set of installed skills/plugins.

### Pattern 6: Copying Files from Host

**Examples**: Claude Code skills, SSH keys, certificates, config templates

Use `$SILO_HOST_HOME` to reference the real host HOME directory:

```toml
[setup]
on_init = [
  "mkdir -p $HOME/.claude/skills/ios-pilot",
  "cp $SILO_HOST_HOME/.claude/skills/ios-pilot/SKILL.md $HOME/.claude/skills/ios-pilot/",
]
```

`$SILO_HOST_HOME` is always set to the original host HOME before silo redirected it.

### Pattern 7: Python venv Tools

**Examples**: pipx packages, poetry, custom scripts

Create a venv inside the isolated home:

```toml
[setup]
on_init = [
  "python3 -m venv $HOME/.venv",
  "$HOME/.venv/bin/pip install some-package",
]
```

### Pattern 8: Native Installers (HOME-relative Binaries)

**Examples**: Claude Code (curl installer), Rust/rustup, Deno

Some tools use native installers (e.g. `curl | sh`) that place binaries under `$HOME/.local/bin/` and version data under `$HOME/.local/share/`. Since silo redirects `$HOME`, the tool's self-check fails — it looks for its own binary at the **silo** HOME path, not the real host path.

**Symptom** (Claude Code native install):

```
installMethod is native, but directory ~/.silo/myenv/home/.local/bin does not exist
installMethod is native, but claude command not found at ~/.silo/myenv/home/.local/bin/claude
```

**Fix**: Symlink the host binary into the silo HOME:

```toml
[setup]
on_init = [
  # Link host's native-installed binary so the tool's self-check passes
  "mkdir -p $HOME/.local/bin && ln -sf $SILO_HOST_HOME/.local/bin/claude $HOME/.local/bin/claude",
]
```

**Why symlink, not copy?** The symlink automatically follows host upgrades. When the host updates (e.g. `claude update`), the silo environment picks up the new version without re-running setup.

**Why only the binary, not `$HOME/.local/share/`?** Symlinking the share directory would let the silo process write into the host's version-management directory, breaking isolation. The binary symlink is read-only in practice and safe.

**General pattern** for any native-installed tool:

```toml
[setup]
on_init = [
  "mkdir -p $HOME/.local/bin && ln -sf $SILO_HOST_HOME/.local/bin/<tool> $HOME/.local/bin/<tool>",
]
```

> **npm-installed tools don't need this** — `npm install -g` places binaries in the npm global prefix (e.g. `/usr/local/bin/`), which is on PATH and doesn't depend on `$HOME`.

---

## Complete Example: ios-pilot + lark-cli

A manifest combining multiple tools in one environment:

```toml
id = "ai-dev"
root = "~/.silo/ai-dev"
inherit_cwd = true
shared_paths = []

[env]
allow = ["TERM", "LANG", "LC_ALL", "COLORTERM", "PATH"]
deny = [
  "SSH_AUTH_SOCK",
  "OPENAI_API_KEY",
  "ANTHROPIC_API_KEY",
]

[env.set]
AI_ENV = "ai-dev"

[secrets]
provider = "keychain"
items = ["ANTHROPIC_API_KEY"]

[shell]
program = "/bin/zsh"
init = "env.zsh"

[network]
mode = "default"

[setup]
on_init = [
  # ios-pilot: config init only (binary on host PATH)
  "test -f $XDG_CONFIG_HOME/ios-pilot/config.json || ios-pilot config init",

  # lark-cli: install + config + skills
  "npm ls -g @larksuite/cli 2>/dev/null || npm install -g @larksuite/cli",
  "test -d $XDG_CONFIG_HOME/lark-cli || lark-cli config init",
  "npx skills add larksuite/cli -y -g",

  # Copy Claude Code skills from host
  "mkdir -p $HOME/.claude/skills/ios-pilot",
  "cp -n $SILO_HOST_HOME/.claude/skills/ios-pilot/SKILL.md $HOME/.claude/skills/ios-pilot/",
]
```

### Workflow

```bash
# Create and setup
silo init -e ai-dev
silo setup -e ai-dev

# Interactive auth (one-time)
silo shell -e ai-dev
lark-cli auth login
exit

# Use
silo exec -e ai-dev -- ios-pilot device list
silo exec -e ai-dev -- lark-cli calendar +agenda
```

---

## Writing Idempotent Setup Commands

`silo setup --force` re-runs all `on_init` commands. Write commands so they're safe to repeat:

```bash
# File copy: -n = don't overwrite existing
cp -n $SILO_HOST_HOME/source $HOME/dest

# Conditional execution: only if file doesn't exist
test -f $HOME/.config/tool/config.json || tool config init

# npm: only install if not present
npm ls -g @larksuite/cli 2>/dev/null || npm install -g @larksuite/cli

# Python venv: only create if missing
test -d $HOME/.venv || python3 -m venv $HOME/.venv

# mkdir -p is naturally idempotent
mkdir -p $HOME/.claude/skills/ios-pilot
```

## Tips

- Use `silo show -e <env>` to inspect all resolved environment paths
- Use `silo setup --force` after modifying `on_init` in your manifest
- Interactive operations (OAuth login, browser auth) belong in `silo shell`, not `on_init`
- Check a tool's XDG compliance to predict how well it isolates under silo
- `on_init` commands run with the full silo environment applied (same as `silo exec`)

# silo

[中文文档](README_zh.md)

Lightweight environment isolation for AI CLI tools on macOS.

silo creates isolated execution contexts for different AI identities and accounts. Each environment gets its own HOME, XDG directories, credentials, and configuration — so your work and personal AI accounts never cross-contaminate.

## Install

### Option 1: Pre-built binary (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/HelloiOS2014/silo/main/install.sh | sh
```

Or with custom prefix:

```bash
curl -fsSL https://raw.githubusercontent.com/HelloiOS2014/silo/main/install.sh | sh -s -- --prefix /usr/local
```

The binary will be installed to `<prefix>/bin/silo`.

### Option 2: Build from source

```bash
cargo install --prefix ~/.local --path .
```

Requires Rust 1.85+ (edition 2024).

## Quick Start

```bash
# Create an environment
silo init -e work

# Run a command in isolation
silo exec -e work -- claude

# Enter an isolated shell
silo shell -e work

# List environments
silo ls

# Inspect resolved config
silo show -e work
```

## Commands

### `silo init -e <name>`

Create a new environment at `~/.silo/<name>/` with default manifest, shell init script, and directory structure.

### `silo exec -e <name> [--tty] [--cwd <path>] -- <command> [args...]`

Execute a command inside the environment. The child process runs with sanitized environment variables, isolated directories, and injected secrets.

```bash
silo exec -e work -- claude
silo exec -e personal -- codex
silo exec -e cn -- bash -lc 'gemini -p "analyze this"'
```

### `silo shell -e <name> [--cwd <path>]`

Enter an interactive shell in the environment. Host shell rc files are suppressed; only the environment's init script is loaded.

### `silo ls`

List all initialized environments.

### `silo show -e <name>`

Display the resolved configuration for an environment, including directories, env vars, secrets provider, and network mode.

## Environment Directory Structure

```
~/.silo/<env-id>/
  manifest.toml      # Environment configuration
  env.zsh            # Shell initialization script
  secrets.env        # Secrets file (mode 600)
  home/              # $HOME
  config/            # $XDG_CONFIG_HOME
  cache/             # $XDG_CACHE_HOME
  data/              # $XDG_DATA_HOME
  state/             # $XDG_STATE_HOME
  tmp/               # $TMPDIR
  run/               # Per-execution runtime directories
```

## Manifest Format

The manifest at `manifest.toml` defines the environment. Only `id`, `root`, and `[env]` are required — all other sections are optional with sensible defaults.

```toml
id = "work"
root = "/Users/you/.silo/work"
inherit_cwd = true                    # default: true
shared_paths = ["/tmp/shared"]        # default: []

[env]
allow = ["TERM", "LANG", "PATH"]      # host vars to inherit
deny = ["OPENAI_API_KEY", "SSH_AUTH_SOCK"]  # vars to block
[env.set]
AI_ENV = "work"                        # fixed vars to inject

[secrets]                              # default: provider = "none"
provider = "keychain"                  # keychain | envfile | none
items = ["OPENAI_API_KEY"]

[shell]                                # default: /bin/zsh + env.zsh
program = "/bin/zsh"
init = "env.zsh"

[network]                              # default: mode = "default"
mode = "default"                       # default | offline | proxy
# proxy_url = "http://proxy:8080"      # required when mode = "proxy"
```

Tilde expansion (`~/`) is supported in `root` and `shared_paths`.

## Secrets

Three providers:

| Provider | Source | Service/Path |
|----------|--------|-------------|
| `keychain` | macOS Keychain | service: `silo.<env-id>`, account: variable name |
| `envfile` | `secrets.env` file | `~/.silo/<env-id>/secrets.env` (must be mode 600) |
| `none` | No secrets | Default when `[secrets]` is omitted |

Add a keychain secret:

```bash
security add-generic-password -s silo.work -a OPENAI_API_KEY -w "sk-..."
```

Or use the envfile:

```bash
echo 'OPENAI_API_KEY=sk-...' >> ~/.silo/work/secrets.env
chmod 600 ~/.silo/work/secrets.env
```

Only keys listed in `items` are injected. Declared but missing keys cause a hard failure.

## Network Mode

| Mode | Behavior |
|------|----------|
| `default` | No network intervention |
| `offline` | Injects `http_proxy`/`https_proxy`/`ALL_PROXY` pointing to a dead proxy (`127.0.0.1:1`) |
| `proxy` | Injects proxy vars with the value from `network.proxy_url` |

## Environment Variable Rules

The child process starts with a **clean** environment (not inherited from host). Variables are built in this order:

1. **Allow list** — inherited from host, filtered by deny list
2. **env.set** — fixed values from manifest
3. **Secrets** — from keychain/envfile
4. **Network mode** — proxy vars for offline/proxy modes
5. **Forced variables** (always set, cannot be overridden):
   - `HOME`, `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`, `XDG_DATA_HOME`, `XDG_STATE_HOME`, `TMPDIR` — point to environment directories
   - `SILO_ROOT` — path to `~/.silo` (preserved across nested execution)
   - `SILO_EXEC_DIR` — per-execution run directory

## Guides

- [Configure Claude Code with different API providers (MiniMax, Kimi, etc.)](docs/guide-claude-code-providers.md)

## License

MIT

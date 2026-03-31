# Configure Claude Code with Different API Providers in silo

[中文版](guide-claude-code-providers_zh.md)

This guide shows how to use silo to run Claude Code with different API providers (MiniMax, Kimi, Anthropic) in isolated environments, so each provider gets its own credentials, configuration, and conversation history.

## Why silo?

Claude Code configuration depends on environment variables and files under `~/.claude/`. If you use multiple API sources (e.g. official Anthropic for work, MiniMax for personal, Kimi for testing), switching requires changing environment variables and config files each time.

silo creates an independent HOME and config directory for each source. One command to switch, zero cross-contamination.

## Prerequisites

- silo installed (`cargo install --path .`)
- Claude Code installed (`npm install -g @anthropic-ai/claude-code`)
- API keys for your chosen providers

## Setting Up MiniMax

MiniMax provides an Anthropic-compatible API endpoint.

### 1. Create the environment

```bash
silo init -e minimax
```

### 2. Edit the manifest

Edit `~/.silo/minimax/manifest.toml`:

```toml
id = "minimax"
root = "/Users/yourname/.silo/minimax"

[env]
allow = ["PATH", "TERM", "LANG", "LC_ALL", "COLORTERM"]
deny = [
  "ANTHROPIC_API_KEY",
  "ANTHROPIC_AUTH_TOKEN",
  "ANTHROPIC_BASE_URL",
  "OPENAI_API_KEY",
]

[env.set]
ANTHROPIC_BASE_URL = "https://api.minimaxi.com/anthropic"
ANTHROPIC_MODEL = "MiniMax-M2.7"
ANTHROPIC_SMALL_FAST_MODEL = "MiniMax-M2.7"
ANTHROPIC_DEFAULT_SONNET_MODEL = "MiniMax-M2.7"
ANTHROPIC_DEFAULT_OPUS_MODEL = "MiniMax-M2.7"
ANTHROPIC_DEFAULT_HAIKU_MODEL = "MiniMax-M2.7"
API_TIMEOUT_MS = "3000000"
CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = "1"

[secrets]
provider = "envfile"
items = ["ANTHROPIC_AUTH_TOKEN"]

[network]
mode = "default"
```

### 3. Configure the secret

```bash
echo 'ANTHROPIC_AUTH_TOKEN=your-minimax-api-key' > ~/.silo/minimax/secrets.env
chmod 600 ~/.silo/minimax/secrets.env
```

### 4. Initialize Claude Code config

First run requires creating the onboarding marker in the environment's HOME:

```bash
silo exec -e minimax -- mkdir -p \$HOME/.claude
silo exec -e minimax -- bash -c 'echo "{\"hasCompletedOnboarding\": true}" > $HOME/.claude.json'
```

### 5. Launch

```bash
silo exec -e minimax -- claude
```

## Setting Up Kimi

Kimi (Moonshot AI) also provides an Anthropic-compatible Claude Code endpoint.

### 1. Create the environment

```bash
silo init -e kimi
```

### 2. Edit the manifest

Edit `~/.silo/kimi/manifest.toml`:

```toml
id = "kimi"
root = "/Users/yourname/.silo/kimi"

[env]
allow = ["PATH", "TERM", "LANG", "LC_ALL", "COLORTERM"]
deny = [
  "ANTHROPIC_API_KEY",
  "ANTHROPIC_AUTH_TOKEN",
  "ANTHROPIC_BASE_URL",
  "OPENAI_API_KEY",
]

[env.set]
ANTHROPIC_BASE_URL = "https://api.kimi.com/coding/"
ENABLE_TOOL_SEARCH = "false"
CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = "1"

[secrets]
provider = "envfile"
items = ["ANTHROPIC_API_KEY"]

[network]
mode = "default"
```

### 3. Configure the secret

```bash
echo 'ANTHROPIC_API_KEY=sk-kimi-your-key' > ~/.silo/kimi/secrets.env
chmod 600 ~/.silo/kimi/secrets.env
```

### 4. Initialize Claude Code config

```bash
silo exec -e kimi -- mkdir -p \$HOME/.claude
silo exec -e kimi -- bash -c 'echo "{\"hasCompletedOnboarding\": true}" > $HOME/.claude.json'
```

### 5. Launch

```bash
silo exec -e kimi -- claude
```

Type `/status` after launch to confirm the model is correctly configured.

## Setting Up Official Anthropic

To manage the official Anthropic account through silo as well:

### 1. Create the environment

```bash
silo init -e anthropic
```

### 2. Edit the manifest

Edit `~/.silo/anthropic/manifest.toml`:

```toml
id = "anthropic"
root = "/Users/yourname/.silo/anthropic"

[env]
allow = ["PATH", "TERM", "LANG", "LC_ALL", "COLORTERM"]
deny = ["OPENAI_API_KEY"]

[env.set]
AI_ENV = "anthropic"

[secrets]
provider = "envfile"
items = ["ANTHROPIC_API_KEY"]

[network]
mode = "default"
```

### 3. Configure the secret

```bash
echo 'ANTHROPIC_API_KEY=sk-ant-your-key' > ~/.silo/anthropic/secrets.env
chmod 600 ~/.silo/anthropic/secrets.env
```

### 4. Launch

```bash
silo exec -e anthropic -- claude
```

## Configuring MCP Servers

Since silo isolates HOME, each environment has its own `~/.claude/settings.json`. MCP server configurations are automatically isolated per environment — a server configured in `minimax` won't appear in `kimi`.

### MiniMax MCP Server

MiniMax provides an MCP server with **web search** and **image understanding** capabilities.

**Prerequisites:** [uv](https://docs.astral.sh/uv/) must be installed on the host machine. If you already have it, there's no need to install it again inside the silo environment — silo inherits PATH from the host by default, so tools like `uv`, `uvx`, `npx`, `node`, and `python` are all available inside isolated environments.

```bash
# Install uv on the host (skip if already installed)
curl -LsSf https://astral.sh/uv/install.sh | sh
```

**Option 1: Via `silo shell` + `claude mcp add`**

```bash
silo shell -e minimax

# Inside the isolated shell:
claude mcp add minimax -- uvx minimax-coding-plan-mcp -y

exit
```

Then manually add the environment variables to the MCP config. Edit `~/.silo/minimax/home/.claude/settings.json`:

```json
{
  "mcpServers": {
    "minimax": {
      "command": "uvx",
      "args": ["minimax-coding-plan-mcp", "-y"],
      "env": {
        "MINIMAX_API_KEY": "your-minimax-api-key",
        "MINIMAX_API_HOST": "https://api.minimaxi.com"
      }
    }
  }
}
```

**Option 2: Edit settings.json directly**

```bash
mkdir -p ~/.silo/minimax/home/.claude
cat > ~/.silo/minimax/home/.claude/settings.json << 'EOF'
{
  "mcpServers": {
    "minimax": {
      "command": "uvx",
      "args": ["minimax-coding-plan-mcp", "-y"],
      "env": {
        "MINIMAX_API_KEY": "your-minimax-api-key",
        "MINIMAX_API_HOST": "https://api.minimaxi.com"
      }
    }
  }
}
EOF
```

**Verify:** Launch Claude Code and type `/mcp` to confirm the MiniMax tools are available:

```bash
silo exec -e minimax -- claude
# Then type: /mcp
```

You should see `web_search` and `understand_image` listed.

> **Note:** MCP server API keys are passed through the MCP config's `env` field, not through silo's manifest secrets. This is because Claude Code reads MCP env vars from its own settings, not from the process environment.

### Other Provider MCP Servers

The same pattern works for any MCP server. Enter the environment's shell, add the MCP server, and the configuration stays isolated:

```bash
silo shell -e <env-name>
claude mcp add <server-name> -- <command> [args...]
exit
```

Or directly edit `~/.silo/<env-name>/home/.claude/settings.json`.

## Daily Usage

Once configured, switching is a single command:

```bash
# Use MiniMax
silo exec -e minimax -- claude

# Use Kimi
silo exec -e kimi -- claude

# Use official Anthropic
silo exec -e anthropic -- claude

# List all environments
silo ls

# Inspect an environment's config
silo show -e minimax
```

Each environment's Claude Code configuration, conversation history, and cache are completely independent.

## Interactive Shell

If you need to do more inside an isolated environment (e.g. install MCP tools, modify Claude Code settings):

```bash
silo shell -e minimax
# You are now in the minimax isolated environment
claude settings  # Modify Claude Code settings
exit             # Leave
```

## Using Keychain Instead of envfile

If you prefer macOS Keychain over plaintext files for secrets:

```bash
# Add the secret to Keychain
security add-generic-password -s silo.minimax -a ANTHROPIC_AUTH_TOKEN -w "your-api-key"
```

Then change `[secrets]` in the manifest to:

```toml
[secrets]
provider = "keychain"
items = ["ANTHROPIC_AUTH_TOKEN"]
```

Keychain is more secure — secrets are encrypted by the system and never appear as plaintext on disk.

## Troubleshooting

**Claude Code prompts for login or onboarding:**
Ensure `$HOME/.claude.json` was created (step 4).

**API request fails (401/403):**
Check the secret is set correctly: `silo exec -e minimax -- printenv ANTHROPIC_AUTH_TOKEN`

**Environment variable leakage:**
Run `silo show -e minimax` to inspect the config. Ensure sensitive variables are in the deny list.

**Host secrets polluting the isolated environment:**
Confirm sensitive variables are in the deny list, not the allow list. silo starts child processes with a clean environment — only variables in the allow list are inherited from the host.

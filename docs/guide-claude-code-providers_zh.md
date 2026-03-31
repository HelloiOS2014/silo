# 在 silo 中为不同环境配置不同的 Claude Code 源

[English version](guide-claude-code-providers.md)

本教程介绍如何使用 silo 为 Claude Code 配置不同的 API 提供商（MiniMax、Kimi 等），实现多账号/多源隔离。

## 为什么需要 silo？

Claude Code 的配置依赖环境变量和 `~/.claude/` 目录下的文件。如果你同时使用多个 API 源（比如工作用官方 Anthropic、个人用 MiniMax、测试用 Kimi），直接切换需要反复修改环境变量和配置文件。

silo 为每个源创建独立的 HOME 和配置目录，互不干扰，一条命令切换。

## 前置条件

- 已安装 silo（`cargo install --path .`）
- 已安装 Claude Code（`npm install -g @anthropic-ai/claude-code`）
- 已获取各平台的 API Key

## 配置 MiniMax 环境

MiniMax 通过兼容 Anthropic API 的方式提供服务。

### 1. 创建环境

```bash
silo init -e minimax
```

### 2. 编辑 manifest

编辑 `~/.silo/minimax/manifest.toml`：

```toml
id = "minimax"
root = "/Users/你的用户名/.silo/minimax"

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

### 3. 配置密钥

```bash
echo 'ANTHROPIC_AUTH_TOKEN=你的MiniMax API Key' > ~/.silo/minimax/secrets.env
chmod 600 ~/.silo/minimax/secrets.env
```

### 4. 初始化 Claude Code 配置

首次使用需要在环境的 HOME 下创建 Claude Code 的初始化标记：

```bash
silo exec -e minimax -- mkdir -p \$HOME/.claude
silo exec -e minimax -- bash -c 'echo "{\"hasCompletedOnboarding\": true}" > $HOME/.claude.json'
```

### 5. 启动

```bash
silo exec -e minimax -- claude
```

## 配置 Kimi 环境

Kimi（月之暗面）也提供兼容 Anthropic API 的 Claude Code 接入。

### 1. 创建环境

```bash
silo init -e kimi
```

### 2. 编辑 manifest

编辑 `~/.silo/kimi/manifest.toml`：

```toml
id = "kimi"
root = "/Users/你的用户名/.silo/kimi"

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

### 3. 配置密钥

```bash
echo 'ANTHROPIC_API_KEY=sk-kimi-你的Key' > ~/.silo/kimi/secrets.env
chmod 600 ~/.silo/kimi/secrets.env
```

### 4. 初始化 Claude Code 配置

```bash
silo exec -e kimi -- mkdir -p \$HOME/.claude
silo exec -e kimi -- bash -c 'echo "{\"hasCompletedOnboarding\": true}" > $HOME/.claude.json'
```

### 5. 启动

```bash
silo exec -e kimi -- claude
```

启动后输入 `/status` 确认模型已正确配置。

## 配置官方 Anthropic 环境

如果你也想把官方 Anthropic 账号纳入 silo 管理：

### 1. 创建环境

```bash
silo init -e anthropic
```

### 2. 编辑 manifest

编辑 `~/.silo/anthropic/manifest.toml`：

```toml
id = "anthropic"
root = "/Users/你的用户名/.silo/anthropic"

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

### 3. 配置密钥

```bash
echo 'ANTHROPIC_API_KEY=sk-ant-你的Key' > ~/.silo/anthropic/secrets.env
chmod 600 ~/.silo/anthropic/secrets.env
```

### 4. 启动

```bash
silo exec -e anthropic -- claude
```

## 配置 MCP Server

由于 silo 隔离了 HOME，每个环境有自己独立的 `~/.claude/settings.json`，MCP 配置天然隔离——在 `minimax` 环境配的 MCP server 不会出现在 `kimi` 环境中。

### MiniMax MCP Server

MiniMax 提供专属 MCP server，具备**网络搜索**和**图片理解**能力。

**前置条件：** 宿主机需要安装 [uv](https://docs.astral.sh/uv/)。如果已经装过，**不需要在隔离环境中重新安装**——silo 默认继承宿主机的 PATH，`uv`、`uvx`、`npx`、`node`、`python` 等工具在隔离环境中直接可用。

```bash
# 在宿主机安装 uv（已安装则跳过）
curl -LsSf https://astral.sh/uv/install.sh | sh
```

**方式一：通过 `silo shell` + `claude mcp add`**

```bash
silo shell -e minimax

# 在隔离 shell 中：
claude mcp add minimax -- uvx minimax-coding-plan-mcp -y

exit
```

然后手动添加环境变量到 MCP 配置。编辑 `~/.silo/minimax/home/.claude/settings.json`：

```json
{
  "mcpServers": {
    "minimax": {
      "command": "uvx",
      "args": ["minimax-coding-plan-mcp", "-y"],
      "env": {
        "MINIMAX_API_KEY": "你的MiniMax API Key",
        "MINIMAX_API_HOST": "https://api.minimaxi.com"
      }
    }
  }
}
```

**方式二：直接编辑 settings.json**

```bash
mkdir -p ~/.silo/minimax/home/.claude
cat > ~/.silo/minimax/home/.claude/settings.json << 'EOF'
{
  "mcpServers": {
    "minimax": {
      "command": "uvx",
      "args": ["minimax-coding-plan-mcp", "-y"],
      "env": {
        "MINIMAX_API_KEY": "你的MiniMax API Key",
        "MINIMAX_API_HOST": "https://api.minimaxi.com"
      }
    }
  }
}
EOF
```

**验证：** 启动 Claude Code 后输入 `/mcp` 确认 MiniMax 工具可用：

```bash
silo exec -e minimax -- claude
# 输入: /mcp
```

应该能看到 `web_search` 和 `understand_image` 两个工具。

> **说明：** MCP server 的 API Key 通过 MCP 配置的 `env` 字段传递，不需要加到 silo manifest 的 secrets 中。这是因为 Claude Code 从自己的 settings 读取 MCP 环境变量，而非从进程环境中读取。

### 其他厂商的 MCP Server

同样的模式适用于任何 MCP server。进入环境的 shell，添加 MCP server，配置自动隔离：

```bash
silo shell -e <环境名>
claude mcp add <server名> -- <命令> [参数...]
exit
```

或直接编辑 `~/.silo/<环境名>/home/.claude/settings.json`。

## 日常使用

配置完成后，日常切换只需一条命令：

```bash
# 用 MiniMax 源
silo exec -e minimax -- claude

# 用 Kimi 源
silo exec -e kimi -- claude

# 用官方 Anthropic
silo exec -e anthropic -- claude

# 查看所有环境
silo ls

# 查看某个环境的配置
silo show -e minimax
```

每个环境的 Claude Code 配置、对话历史、缓存完全独立，互不影响。

## 进入交互式 shell

如果你需要在隔离环境中做更多操作（比如安装 MCP 工具、修改 Claude Code 设置）：

```bash
silo shell -e minimax
# 现在你在 minimax 的隔离环境中
claude settings  # 修改 Claude Code 设置
exit             # 退出
```

## 使用 Keychain 代替 envfile

如果你偏好使用 macOS 钥匙串而非文件存储密钥：

```bash
# 添加密钥到钥匙串
security add-generic-password -s silo.minimax -a ANTHROPIC_AUTH_TOKEN -w "你的API Key"
```

然后将 manifest 中的 `[secrets]` 改为：

```toml
[secrets]
provider = "keychain"
items = ["ANTHROPIC_AUTH_TOKEN"]
```

钥匙串方式更安全——密钥由系统加密存储，不会以明文出现在文件中。

## 故障排查

**Claude Code 提示未登录或需要 onboarding：**
确认已创建 `$HOME/.claude.json`（步骤 4）。

**API 请求失败（401/403）：**
检查密钥是否正确：`silo exec -e minimax -- printenv ANTHROPIC_AUTH_TOKEN`

**环境变量泄露：**
用 `silo show -e minimax` 检查配置，确认 deny 列表包含了不应继承的变量。

**宿主机密钥污染隔离环境：**
确认敏感变量在 deny 列表中，而不是在 allow 列表中。silo 的子进程从空白环境启动，只有 allow 列表中的变量才会被继承。

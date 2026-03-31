# 在 silo 中为不同环境配置不同的 Claude Code 源

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

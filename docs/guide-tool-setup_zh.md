# 在 Silo 隔离环境中安装和配置工具

本指南介绍如何在 silo 隔离环境中安装和配置各类 CLI 工具。

## Silo 的工具隔离原理

| 机制 | 效果 |
|------|------|
| `PATH` 透传 | 宿主机上的二进制文件无需 per-env 安装即可使用 |
| `HOME` 重定向 | `$HOME` → `{env_root}/home` |
| `XDG_*` 重定向 | `$XDG_CONFIG_HOME` → `{env_root}/config`，以此类推 |
| `SILO_HOST_HOME` | 指向宿主机的**真实** HOME（用于从宿主机复制文件） |
| `[setup].on_init` | 声明式的 per-env 初始化命令 |

当你执行 `silo exec -e myenv -- some-tool` 时，工具看到的是完全不同的 HOME 和配置目录。符合 XDG 规范的工具会自动将配置写入隔离位置。

## 快速上手

```bash
# 1. 在宿主机上安装工具（一次性）
# 2. 创建 silo 环境
silo init -e myenv

# 3. 编辑 manifest 添加 setup 钩子
#    ~/.silo/myenv/manifest.toml → 添加 [setup] 段

# 4. 运行 setup
silo setup -e myenv

# 5. 使用
silo exec -e myenv -- some-tool ...
```

---

## 安装模式

### 模式一：静态二进制 CLI（Go/Rust 单文件）

**代表工具**: ios-pilot, rg, fd, jq, gh

二进制文件在宿主机 PATH 上，配置符合 XDG 规范，自动隔离。

```toml
[setup]
on_init = [
  "tool config init",
]
```

无需 per-env 安装二进制，通过 PATH 直接可用。

### 模式二：带 Daemon/Socket 的 CLI

**代表工具**: ios-pilot（daemon + Unix socket）、Docker

二进制共享，daemon socket 路径跟随 `$XDG_CONFIG_HOME` 或 `$TMPDIR`，每个环境独立 daemon 实例。

```toml
[setup]
on_init = [
  "ios-pilot config init",
  "ios-pilot wda setup",
]
```

**注意**: 如果工具硬编码 `~/.config/`，由于 `$HOME` 已被重定向，socket 仍然会落在隔离位置。可通过检查工具的 socket 路径解析逻辑来确认。

### 模式三：npm 全局包

**代表工具**: lark-cli（`@larksuite/cli`）、typescript、eslint

`npm install -g` 写入 `$HOME` 相关路径。由于 HOME 被重定向，每个 silo 环境拥有独立的 npm global prefix 和包目录。

```toml
[env]
allow = ["PATH"]

[env.set]
AI_ENV = "my-lark-env"
# 确保 npm 全局 bin 在 PATH 中
NPM_CONFIG_PREFIX = "$HOME/.npm-global"

[setup]
on_init = [
  "npm config set prefix $HOME/.npm-global",
  "npm install -g @larksuite/cli",
  "lark-cli config init",
]
```

**要点**:
- `npm install -g` 安装到隔离的 HOME 下
- 可能需要配置 `NPM_CONFIG_PREFIX` 并将其 `bin/` 加入 PATH
- 每个环境独立维护自己的包版本

### 模式四：带 OAuth 认证的工具

**代表工具**: lark-cli（`lark-cli auth login`）、gh（GitHub CLI）

OAuth 登录为每个环境创建独立会话。认证 token 存储在配置目录中（通过 XDG 重定向隔离）。

```toml
[setup]
on_init = [
  "npm install -g @larksuite/cli",
  "lark-cli config init",
]
```

setup 完成后，手动进行交互式登录：

```bash
silo shell -e my-lark-env
lark-cli auth login
# 在浏览器中完成 OAuth 流程
exit
```

**Keychain 隔离注意**: macOS Keychain 是系统级的，不按环境隔离。如果工具将 token 存在 Keychain（而非配置文件）中，不同 silo 环境可能共享认证状态。解决方案：
- 检查工具是否支持基于文件的 token 存储
- 使用不同的 Keychain service name（如果工具按 app ID 区分）
- 对于 lark-cli：不同环境配置不同的飞书应用

**经验法则**: 交互式认证（浏览器 OAuth）应通过 `silo shell` 手动完成，不要放在 `on_init` 中。

### 模式五：带 Skill/Plugin 生态的工具

**代表工具**: lark-cli（skills）、Claude Code（MCP plugins）

除了主程序，还需安装额外的 skill 包或 plugin。

```toml
[setup]
on_init = [
  "npm install -g @larksuite/cli",
  "npx skills add larksuite/cli -y -g",
  "lark-cli config init",
]
```

每个 silo 环境维护自己独立的 skill/plugin 集合。

### 模式六：从宿主机复制文件

**代表工具**: Claude Code skills、SSH 密钥、证书、配置模板

使用 `$SILO_HOST_HOME` 引用宿主机的真实 HOME 目录：

```toml
[setup]
on_init = [
  "mkdir -p $HOME/.claude/skills/ios-pilot",
  "cp -n $SILO_HOST_HOME/.claude/skills/ios-pilot/SKILL.md $HOME/.claude/skills/ios-pilot/ 2>/dev/null || true",
]
```

`$SILO_HOST_HOME` 始终指向 silo 重定向之前的宿主机原始 HOME。

### 模式七：Python venv 工具

**代表工具**: pipx 包、poetry、自定义脚本

在隔离的 home 目录中创建 venv：

```toml
[setup]
on_init = [
  "python3 -m venv $HOME/.venv",
  "$HOME/.venv/bin/pip install some-package",
]
```

### 模式八：Native 安装器（二进制文件在 HOME 下）

**代表工具**: Claude Code（curl 安装脚本）、Rust/rustup、Deno

某些工具通过原生安装脚本（如 `curl | sh`）将二进制文件安装到 `$HOME/.local/bin/`，版本数据放在 `$HOME/.local/share/`。由于 silo 重定向了 `$HOME`，工具的自检会失败——它在 silo 的 HOME 路径下找不到自己的二进制文件。

**典型报错**（以 Claude Code native 安装为例）：

```
installMethod is native, but directory ~/.silo/myenv/home/.local/bin does not exist
installMethod is native, but claude command not found at ~/.silo/myenv/home/.local/bin/claude
```

**修复方法**（两步）：

**第一步**：将宿主机的二进制文件符号链接到 silo HOME 下：

```toml
[setup]
on_init = [
  # 链接宿主机 native 安装的二进制，确保工具自检通过
  "mkdir -p $HOME/.local/bin && ln -sf $SILO_HOST_HOME/.local/bin/claude $HOME/.local/bin/claude",
]
```

**第二步**：通过 `env.prepend` 将 `$HOME/.local/bin` 加入 PATH：

```toml
[env.prepend]
PATH = "$HOME/.local/bin"
```

`env.prepend` 支持 `$VAR` 变量展开——`$HOME` 会展开为 silo 的 HOME 路径，而非宿主机 HOME。值以 `:` 为分隔符前置到已有变量。

**为什么用 symlink 而不是 copy？** 符号链接会自动跟随宿主机的升级。当宿主机更新工具版本（如 `claude update`）后，silo 环境无需重新 setup 即可使用新版本。

**为什么只链接二进制，不链接 `$HOME/.local/share/`？** 链接 share 目录会让 silo 中的进程写入宿主机的版本管理目录，破坏隔离性。只链接二进制文件实际上是只读访问，不影响隔离。

**通用写法**（适用于任何 native 安装的工具）：

```toml
[env.prepend]
PATH = "$HOME/.local/bin"

[setup]
on_init = [
  "mkdir -p $HOME/.local/bin && ln -sf $SILO_HOST_HOME/.local/bin/<工具名> $HOME/.local/bin/<工具名>",
]
```

> **npm 安装的工具不需要这样做**——`npm install -g` 将二进制放在 npm 全局 prefix（如 `/usr/local/bin/`），在 PATH 上且不依赖 `$HOME`。

---

## 完整范例：ios-pilot + lark-cli 组合环境

一个在同一环境中配置多个工具的 manifest：

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
  # ios-pilot：仅初始化配置（二进制在宿主机 PATH 上）
  "test -f $XDG_CONFIG_HOME/ios-pilot/config.json || ios-pilot config init",

  # lark-cli：安装 + 配置 + skills
  "npm ls -g @larksuite/cli 2>/dev/null || npm install -g @larksuite/cli",
  "test -d $XDG_CONFIG_HOME/lark-cli || lark-cli config init",
  "npx skills add larksuite/cli -y -g",

  # 从宿主机复制 Claude Code skills
  "mkdir -p $HOME/.claude/skills/ios-pilot",
  "cp -n $SILO_HOST_HOME/.claude/skills/ios-pilot/SKILL.md $HOME/.claude/skills/ios-pilot/",
]
```

### 使用流程

```bash
# 创建并初始化
silo init -e ai-dev
silo setup -e ai-dev

# 交互式登录（一次性）
silo shell -e ai-dev
lark-cli auth login
exit

# 使用
silo exec -e ai-dev -- ios-pilot device list
silo exec -e ai-dev -- lark-cli calendar +agenda
```

---

## 幂等写法参考

`silo setup --force` 会重新执行所有 `on_init` 命令。推荐使用幂等写法，确保重跑不破坏已有配置：

```bash
# 文件复制：-n 表示不覆盖已有文件
# 注意：macOS 的 cp -n 在文件已存在时返回 exit 1，需加 || true
cp -n $SILO_HOST_HOME/source $HOME/dest 2>/dev/null || true

# 条件执行：仅在文件不存在时才执行
test -f $HOME/.config/tool/config.json || tool config init

# npm：仅在未安装时才安装
npm ls -g @larksuite/cli 2>/dev/null || npm install -g @larksuite/cli

# Python venv：仅在不存在时创建
test -d $HOME/.venv || python3 -m venv $HOME/.venv

# 目录创建：mkdir -p 天然幂等
mkdir -p $HOME/.claude/skills/ios-pilot
```

## 使用技巧

- 用 `silo show -e <env>` 查看所有解析后的环境路径
- 修改 `on_init` 后用 `silo setup --force` 重跑
- 交互式操作（OAuth 登录、浏览器认证）应通过 `silo shell` 手动完成，不放在 `on_init` 中
- 检查工具的 XDG 合规性来预测在 silo 下的隔离效果
- `on_init` 命令在完整的 silo 隔离环境中执行（与 `silo exec` 相同）

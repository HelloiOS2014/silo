# silo

macOS 上轻量级的 AI CLI 环境隔离工具。

silo 为不同的 AI 身份和账号创建独立的执行上下文。每个环境拥有自己的 HOME、XDG 目录、凭证和配置——让你的工作账号和个人账号永远不会相互干扰。

## 安装

### 方式一：下载预编译二进制文件（推荐）

```bash
curl -fsSL https://raw.githubusercontent.com/HelloiOS2014/silo/main/install.sh | sh
```

或指定安装路径：

```bash
curl -fsSL https://raw.githubusercontent.com/HelloiOS2014/silo/main/install.sh | sh -s -- --prefix /usr/local
```

二进制文件将安装到 `<prefix>/bin/silo`。

### 方式二：从源码构建

```bash
cargo build --release && cp target/release/silo ~/.local/bin/silo
```

需要 Rust 1.85+（edition 2024）。

## 快速上手

```bash
# 创建环境
silo init -e work

# 在隔离环境中执行命令
silo exec -e work -- claude

# 进入隔离 shell
silo shell -e work

# 列出所有环境
silo ls

# 查看生效配置
silo show -e work
```

## 命令参考

### `silo init -e <名称>`

在 `~/.silo/<名称>/` 创建新环境，包含默认 manifest、shell 初始化脚本和完整目录结构。

### `silo exec -e <名称> [--tty] [--cwd <路径>] -- <命令> [参数...]`

在环境中执行命令。子进程使用净化后的环境变量、隔离的目录和注入的密钥运行。

```bash
silo exec -e work -- claude
silo exec -e personal -- codex
silo exec -e cn -- bash -lc 'gemini -p "分析当前目录"'
```

### `silo shell -e <名称> [--cwd <路径>]`

进入环境的交互式 shell。宿主机的 shell 配置文件会被抑制，只加载环境自己的初始化脚本。

### `silo ls`

列出所有已初始化的环境。

### `silo show -e <名称>`

展示环境的完整生效配置，包括目录映射、环境变量、密钥提供者和网络模式。

## 环境目录结构

```
~/.silo/<env-id>/
  manifest.toml      # 环境配置
  env.zsh            # Shell 初始化脚本
  secrets.env        # 密钥文件（权限 600）
  home/              # $HOME
  config/            # $XDG_CONFIG_HOME
  cache/             # $XDG_CACHE_HOME
  data/              # $XDG_DATA_HOME
  state/             # $XDG_STATE_HOME
  tmp/               # $TMPDIR
  run/               # 每次执行的运行时目录
```

## Manifest 格式

环境由 `manifest.toml` 定义。只有 `id`、`root` 和 `[env]` 是必填项，其他段可选，均有合理默认值。

```toml
id = "work"
root = "/Users/you/.silo/work"
inherit_cwd = true                    # 默认: true
shared_paths = ["/tmp/shared"]        # 默认: []

[env]
allow = ["TERM", "LANG", "PATH"]      # 从宿主机继承的变量
deny = ["OPENAI_API_KEY", "SSH_AUTH_SOCK"]  # 强制阻止的变量
[env.set]
AI_ENV = "work"                        # 固定注入的变量

[secrets]                              # 默认: provider = "none"
provider = "keychain"                  # keychain | envfile | none
items = ["OPENAI_API_KEY"]

[shell]                                # 默认: /bin/zsh + env.zsh
program = "/bin/zsh"
init = "env.zsh"

[network]                              # 默认: mode = "default"
mode = "default"                       # default | offline | proxy
# proxy_url = "http://proxy:8080"      # mode = "proxy" 时必填
```

`root` 和 `shared_paths` 支持波浪号展开（`~/`）。

## 密钥管理

支持三种提供者：

| 提供者 | 来源 | 服务/路径 |
|--------|------|-----------|
| `keychain` | macOS 钥匙串 | 服务名: `silo.<env-id>`，账户名: 变量名 |
| `envfile` | `secrets.env` 文件 | `~/.silo/<env-id>/secrets.env`（权限必须为 600） |
| `none` | 不使用密钥 | 省略 `[secrets]` 段时的默认值 |

添加钥匙串密钥：

```bash
security add-generic-password -s silo.work -a OPENAI_API_KEY -w "sk-..."
```

或使用 envfile：

```bash
echo 'OPENAI_API_KEY=sk-...' >> ~/.silo/work/secrets.env
chmod 600 ~/.silo/work/secrets.env
```

envfile 支持 dotenv 格式：`#` 注释、`export` 前缀、单双引号、双引号内转义。

只有 `items` 中声明的 key 会被注入。声明了但找不到的 key 会直接报错，不会静默跳过。

## 网络模式

| 模式 | 行为 |
|------|------|
| `default` | 不干预网络 |
| `offline` | 注入 `http_proxy`/`https_proxy`/`ALL_PROXY` 指向不存在的代理（`127.0.0.1:1`），令 HTTP 请求失败 |
| `proxy` | 注入代理变量，值为 `network.proxy_url` 指定的地址 |

## 环境变量规则

子进程从**空白环境**启动（不继承宿主机环境）。变量按以下顺序构建：

1. **Allow 白名单** — 从宿主机继承，被 deny 列表过滤
2. **env.set** — manifest 中定义的固定值
3. **Secrets** — 从 keychain/envfile 解析的密钥
4. **网络模式** — offline/proxy 时注入代理变量
5. **强制变量**（最后写入，不可覆盖）：
   - `HOME`、`XDG_CONFIG_HOME`、`XDG_CACHE_HOME`、`XDG_DATA_HOME`、`XDG_STATE_HOME`、`TMPDIR` — 指向环境目录
   - `SILO_ROOT` — `~/.silo` 的路径（嵌套执行时保留已有值）
   - `SILO_EXEC_DIR` — 本次执行的运行时目录

## 使用教程

- [在隔离环境中配置不同源的 Claude Code（MiniMax、Kimi 等）](docs/guide-claude-code-providers_zh.md)

## 许可证

MIT

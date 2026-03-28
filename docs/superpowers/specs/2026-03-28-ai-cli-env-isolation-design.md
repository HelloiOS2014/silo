# AI CLI 轻量环境隔离设计

日期：2026-03-28

## 目标

在 macOS 上提供一层轻量环境执行器，用来为不同 AI 身份/账号提供独立运行环境。

必须满足：

- 主环境的宿主机使用方式完全不变
- 一个身份/账号对应一个侧环境
- 侧环境可以运行任意命令，不和具体工具绑定
- 侧环境同时支持交互式 TUI 和非交互式批处理
- 侧环境从当前工作目录启动，但使用自己的配置、缓存、临时目录和凭证
- 环境不常驻；长期会话由 `tmux` 等工具负责

不追求：

- Docker / Linux namespace 级别的硬隔离
- 每个工具单独做一套专用适配层
- 细粒度到命令、域名、端口的复杂权限系统

## 设计结论

系统提供一个工具无关的本地执行器 `aienv`。

`aienv` 只负责一件事：在指定环境中启动一条命令。

- 主环境：宿主机原样使用，不受 `aienv` 约束
- 侧环境：由 `manifest.toml` 定义
- 运行时：切换 `HOME/XDG/TMPDIR` 等环境根，保留当前 `cwd`
- 命令：可以是 `codex`、`claude`、`gemini`，也可以是任意 `bash` 命令

## 核心边界

### 隔离边界

V1 的隔离是“粗粒度环境隔离”，核心解决：

- 账号配置串用
- 密钥串用
- 缓存和临时文件串用
- 多身份任务分发
- 指定共享路径协作

V1 不承诺：

- 同一 macOS 用户下的硬内核级沙箱隔离
- 对任意恶意进程的完全防逃逸能力

### 运行语义

- 侧环境默认从宿主机当前目录启动
- 侧环境只切自己的 `HOME/XDG_CONFIG_HOME/XDG_CACHE_HOME/TMPDIR`
- 侧环境同时支持：
  - 非交互执行
  - 交互式 PTY/TUI
  - 直接进入该环境 shell
- 环境本身不常驻
- 长期会话由环境内部的 `tmux` 负责

## 命令接口

V1 只提供以下命令：

```bash
aienv exec --env <id> [--tty] [--cwd <path>] -- <cmd> [args...]
aienv shell --env <id> [--cwd <path>]
aienv ls
aienv show --env <id>
aienv init --env <id>
```

语义：

- `exec`
  - 执行任意命令
  - 默认非交互
- `exec --tty`
  - 执行交互式 TUI/PTY 命令
- `shell`
  - 进入该环境 shell
- `ls`
  - 列出环境
- `show`
  - 展示最终生效配置
- `init`
  - 初始化环境目录和基础文件

示例：

```bash
aienv exec --env work --tty -- claude
aienv exec --env personal --tty -- codex
aienv exec --env cn -- bash -lc 'gemini -p "分析当前目录"'
aienv shell --env work
```

## 环境目录结构

每个环境固定放在：

```text
~/.aienv/<env-id>/
  manifest.toml
  env.zsh
  home/
  config/
  cache/
  tmp/
  run/
```

目录含义：

- `home/`
  - 该环境自己的 `HOME`
- `config/`
  - `XDG_CONFIG_HOME`
- `cache/`
  - `XDG_CACHE_HOME`
- `tmp/`
  - `TMPDIR`
- `run/`
  - pid、pty、socket 等运行时文件
- `env.zsh`
  - 该环境专用 shell 初始化脚本

## Manifest 规范

V1 的 `manifest.toml` 只允许以下字段：

```toml
id = "work"
extends = "base/default"
root = "/Users/you/.aienv/work"
inherit_cwd = true
shared_paths = ["/tmp/ai-bus"]

[env]
allow = ["TERM", "LANG", "LC_ALL", "COLORTERM", "PATH"]
deny = [
  "SSH_AUTH_SOCK",
  "OPENAI_API_KEY",
  "ANTHROPIC_API_KEY",
  "GEMINI_API_KEY",
  "AWS_ACCESS_KEY_ID",
  "AWS_SECRET_ACCESS_KEY",
  "AWS_SESSION_TOKEN",
  "GOOGLE_APPLICATION_CREDENTIALS",
  "http_proxy",
  "https_proxy",
  "ALL_PROXY"
]

[env.set]
AI_ENV = "work"

[secrets]
provider = "keychain"
items = ["OPENAI_API_KEY", "ANTHROPIC_API_KEY", "GEMINI_API_KEY"]

[shell]
program = "/bin/zsh"
init = "env.zsh"

[network]
mode = "default"
```

字段约束：

- `id`
  - 必填，环境名
- `extends`
  - 可选，只允许单层继承
- `root`
  - 必填，环境根目录
- `inherit_cwd`
  - 可选，默认 `true`
- `shared_paths`
  - 可选，固定共享路径
- `env.allow`
  - 允许从宿主机继承的普通变量
- `env.deny`
  - 启动前强制清掉的变量
- `env.set`
  - 固定注入的变量
- `secrets.provider`
  - V1 只支持 `keychain` 或 `envfile`
- `secrets.items`
  - 启动时需要注入的密钥变量名
- `shell.program`
  - `shell` 命令使用的 shell
- `shell.init`
  - 相对 `root` 的初始化脚本
- `network.mode`
  - V1 只允许 `default` / `offline` / `proxy`

V1 明确不支持：

- `tool_profiles`
- 多层继承
- 条件表达式
- 动态 mount 规则
- 复杂权限 DSL
- 审计规则
- hooks

## 环境变量规则

运行子进程时，不继承宿主机整包环境变量；由执行器重建一份新环境。

### 默认保留

- `TERM`
- `LANG`
- `LC_ALL`
- `COLORTERM`
- `PATH`

### 强制重写

- `HOME`
- `XDG_CONFIG_HOME`
- `XDG_CACHE_HOME`
- `TMPDIR`

### 默认清理

- `SSH_AUTH_SOCK`
- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `GEMINI_API_KEY`
- `AWS_*`
- `GCP_*`
- `GOOGLE_*`
- `AZURE_*`
- `http_proxy`
- `https_proxy`
- `ALL_PROXY`

再按 `manifest` 中的 `env.set` 和 `secrets` 注入环境变量。

## 密钥策略

V1 只支持两种来源：

- `keychain`
- `envfile`

推荐默认使用 `keychain`。

推荐命名方式：

- service: `aienv.<env-id>`
- account: 对应环境变量名，例如 `OPENAI_API_KEY`

行为约束：

- 只有 `secrets.items` 里声明的 key 才会被注入
- 声明了但找不到，直接报错退出
- 不允许偷偷回退到宿主机已有变量

## 路径规则

V1 采用粗粒度路径控制：

- 自动允许当前 `cwd`
- 额外允许 `shared_paths` 中声明的固定路径
- 其他路径默认不认为是授权路径

启动前要求：

- 对 `cwd` 和 `shared_paths` 做真实路径解析
- 路径不存在直接报错
- 如果 `cwd` 通过 symlink 指向未授权位置，则拒绝启动

说明：

如果 V1 不引入系统级沙箱，这一层主要是运行时规则，而不是内核级强封锁。

## Shell 与 PTY 规则

### 非交互

`aienv exec --env <id> -- <cmd...>`

- 直接执行目标命令
- 不自动套 shell
- 返回退出码、stdout、stderr

### 交互

`aienv exec --env <id> --tty -- <cmd...>`

- 创建 PTY
- 转发 stdin/stdout/stderr
- 处理窗口大小变化和信号

### Shell

`aienv shell --env <id>`

- 启动 `shell.program`
- 使用受控启动参数
- 只加载该环境自己的 `shell.init`
- 不直接加载宿主机的全局 shell 配置

## 初始化规则

执行 `aienv init --env <id>` 时：

1. 创建环境目录结构
2. 如果缺少 `env.zsh`，写入最小初始化文件
3. 检查 `manifest.toml`
4. 检查 shell 程序是否存在
5. 检查 `shared_paths` 是否存在
6. 检查 secrets provider 是否可用

最小 `env.zsh` 示例：

```sh
export AI_ENV=work
```

## 并发规则

V1 允许同一环境并发运行多个任务，但每次执行必须使用独立运行子目录，避免 runtime 文件冲突。

例如：

```text
~/.aienv/work/run/<exec-id>/
```

不要让多个执行直接共用同一组临时 pid/socket/pty 文件。

## 错误处理

以下情况必须直接失败，不允许自动回退到主环境：

- 环境配置不存在
- 环境未初始化
- 密钥缺失
- `cwd` 不可访问
- `cwd` 或共享路径不合法
- 目标命令不存在

错误输出应尽量直接，避免隐藏真实原因。

## 测试重点

V1 至少覆盖以下验证：

- 切换环境后，`HOME/XDG/TMPDIR` 是否正确
- 当前目录是否正确继承
- 宿主机敏感变量是否被清理
- `keychain` / `envfile` 注入是否只对声明项生效
- `exec` 与 `exec --tty` 行为是否正确分离
- `shell` 是否只加载环境自己的 init
- 多任务并发是否互不踩 runtime 文件
- 缺配置、缺密钥、非法路径时是否正确失败

## V1 范围外

以下内容不放入 V1：

- 工具专用配置系统
- 细粒度网络控制
- 强沙箱
- 常驻会话管理
- 复杂审计
- 多层模板系统

## 结论

V1 方案采用“工具无关的配置驱动环境执行器”：

- 主环境保持原样
- 侧环境按身份划分
- 命令从当前目录启动
- 配置、缓存、临时目录和密钥按环境隔离
- 交互式和非交互式统一由 `aienv` 提供

这套方案足够轻，也足够贴合当前目标，不需要引入 Docker、独立用户或复杂容器体系。

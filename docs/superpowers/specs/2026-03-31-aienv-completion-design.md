# aienv 完善设计

日期：2026-03-31

## 背景

aienv 是一个 Rust CLI 工具，为 macOS 上的不同 AI 身份/账号提供轻量级环境隔离。当前已实现 `init` 和非交互式 `exec`，约 60% 完成。

本文档基于对全部 11 个源文件的逐行审查、设计文档逐条交叉对照，以及真实使用场景走读，定义将 aienv 从半成品推进到完整可用工具所需的全部变更。

共识决策（在头脑风暴阶段确认）：

- TTY 执行采用 inherit stdio 方案，不使用 portable-pty
- 网络模式采用环境变量约束方案（代理注入）
- show 展示解析后的生效配置，不是原始 TOML
- envfile 解析升级为 dotenv 兼容格式

## 变更总览

34 项问题，分为 4 类：

| 类别 | 数量 | 编号 |
|------|------|------|
| Bug 修复与安全加固 | 10 | #5,6,7,8,9,23,24,25,27,28 |
| 未实现命令 | 4 | #1,2,3,4 |
| Manifest 与解析改进 | 8 | #10,11,12,13,14,17,18,33 |
| CLI 体验与清理 | 12 | #19,20,22,26,29,30,31,32,34 及测试补全 |

---

## 一、Bug 修复与安全加固

### 1.1 强制变量不可覆盖（#23）

`build_child_env` 中 7 个强制变量（HOME, XDG_CONFIG_HOME, XDG_CACHE_HOME, XDG_DATA_HOME, XDG_STATE_HOME, TMPDIR, AIENV_ROOT）移到函数**最后写入**，在 env.set 和 secrets 之后。

AIENV_ROOT 的值来自宿主环境（优先读已有 AIENV_ROOT，fallback 到 `$HOME/.aienv`），写入时机在最后但值在最前面就确定了。这样既保留嵌套场景的已有值，又防止 env.set 覆盖。

同时在 `Manifest::validate()` 中增加校验：如果 `env.set` 的 key 或 `secrets.items` 中包含这 7 个保留 key，返回验证错误。双重防御。

### 1.2 exec 接入 secrets（#5）

当前 `exec::run()` 传 `BTreeMap::new()` 作为 secrets，完全忽略 manifest 声明的密钥。

修复：根据 `manifest.secrets.provider` 分发：

- `"keychain"` — 调用 `resolve_from_keychain("aienv.<id>", &items)`
- `"envfile"` — 调用 `resolve_from_envfile(<env-root>/secrets.env, &items)`
- `"none"` — 返回空 BTreeMap
- `items` 为空时直接跳过，不调用 provider

envfile 路径固定约定为 `<env-root>/secrets.env`。文件不存在时报错：

```
secrets.env not found at /Users/panghu/.aienv/work/secrets.env
```

### 1.3 inherit_cwd 生效（#6）

当前 exec 无论 `inherit_cwd` 值如何都使用宿主机 cwd。

修复逻辑：

- `inherit_cwd = true`（默认）：使用 `std::env::current_dir()` 或 `--cwd` 指定值
- `inherit_cwd = false`：默认使用 `manifest.root.join("home")`，`--cwd` 可覆盖

shell 命令同样遵循此逻辑。

### 1.4 信号退出码保留（#28）

当前 `status.code().unwrap_or(1)` 在子进程被信号杀死时丢失信息。

修复（Unix 惯例）：

```rust
#[cfg(unix)]
{
    use std::os::unix::process::ExitStatusExt;
    let code = status.code().unwrap_or_else(|| 128 + status.signal().unwrap_or(1));
    std::process::exit(code);
}
#[cfg(not(unix))]
{
    std::process::exit(status.code().unwrap_or(1));
}
```

### 1.5 嵌套执行支持（#25）

```bash
aienv exec --env work -- aienv exec --env personal -- cmd
```

内层 aienv 读 `$HOME/.aienv/personal/manifest.toml`，但 HOME 已被外层改为 `~/.aienv/work/home`，导致定位失败。

修复：exec 启动子进程时注入 `AIENV_ROOT` 环境变量，值为 aienv 环境根目录的宿主机真实路径。`load_manifest` 和 `aienv_root` 函数优先读 `AIENV_ROOT`，fallback 到 `$HOME/.aienv`。

嵌套场景下保留已有 `AIENV_ROOT`：

```rust
let aienv_root = host.get("AIENV_ROOT")
    .cloned()
    .unwrap_or_else(|| format!("{}/.aienv", host["HOME"]));
```

AIENV_ROOT 作为强制变量注入，不可被 env.set 覆盖。

### 1.6 secrets provider 校验（#8）

`Manifest::validate()` 增加校验：`secrets.provider` 必须是 `"keychain"` | `"envfile"` | `"none"` 之一。

### 1.7 默认 deny 列表补全（#24）

init 生成的默认 manifest deny 列表从当前 4 项补齐为设计文档完整列表：

```toml
deny = [
  "SSH_AUTH_SOCK",
  "OPENAI_API_KEY",
  "ANTHROPIC_API_KEY",
  "GEMINI_API_KEY",
  "AWS_ACCESS_KEY_ID",
  "AWS_SECRET_ACCESS_KEY",
  "AWS_SESSION_TOKEN",
  "GOOGLE_APPLICATION_CREDENTIALS",
  "AZURE_CLIENT_ID",
  "AZURE_CLIENT_SECRET",
  "AZURE_TENANT_ID",
  "http_proxy",
  "https_proxy",
  "ALL_PROXY"
]
```

### 1.8 network.mode 运行时效果（#7）

当前只做 manifest 字符串校验，exec 时无任何处理。

实现：在 `build_child_env` 中，根据 mode 注入代理变量：

| mode | 行为 |
|------|------|
| `"default"` | 不干预 |
| `"offline"` | 注入 `http_proxy=http://127.0.0.1:1`、`https_proxy=http://127.0.0.1:1`、`ALL_PROXY=http://127.0.0.1:1` |
| `"proxy"` | 注入 `http_proxy`/`https_proxy`/`ALL_PROXY` 为 `network.proxy_url` 的值 |

注入时机：在 env.set 之后、强制变量之前。NetworkConfig 扩展：

```rust
pub struct NetworkConfig {
    pub mode: String,
    #[serde(default)]
    pub proxy_url: Option<String>,
}
```

validate 增加：`mode = "proxy"` 时 `proxy_url` 必须有值。

### 1.9 envfile 权限检查（#27）

读取 secrets.env 前检查文件权限：

```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::metadata(path)?.permissions().mode();
    if mode & 0o077 != 0 {
        bail!(
            "secrets.env permissions too open ({:o}), expected 600 or stricter",
            mode & 0o777
        );
    }
}
```

### 1.10 manifest id 与目录名校验（#9）

`load_manifest` 加载后做两项一致性校验：

1. `manifest.id == env_name`，不一致时报错：
```
manifest id "personal" does not match environment name "work"
```

2. 波浪号展开后的 `manifest.root` 与实际环境目录路径一致，不一致时报错：
```
manifest root "/other/path" does not match environment directory "/Users/panghu/.aienv/work"
```

---

## 二、未实现命令

### 2.1 exec --tty（#1）

当前 `todo!()` panic。

实现：由于采用 inherit stdio 方案，`--tty` 与非 `--tty` 走同一条 exec 路径。`.status()` 已经 inherit stdio。`main.rs` 中合并两个分支：

```rust
Commands::Exec { env, tty: _, cwd, command } => {
    let status = commands::exec::run(&env, cwd, command)?;
    // exit code handling...
}
```

`--tty` flag 保留在 CLI 以备将来扩展（真实 PTY 分配），V1 行为等价。移除 `portable-pty` 依赖。

### 2.2 shell（#2）

当前 `todo!()` panic。

新建 `src/commands/shell.rs`。流程：

1. load_manifest
2. resolve_secrets
3. 确定 cwd（遵循 inherit_cwd 逻辑）
4. build_child_env（含 secrets、网络模式、AIENV_ROOT）
5. 构造 shell 启动命令
6. `Command::new().stdin/stdout/stderr(Stdio::inherit()).status()`

shell rc 文件抑制策略——根据 `shell.program` 的文件名（`Path::file_name()`）匹配：

| shell 文件名 | 启动参数 |
|-------------|---------|
| `zsh` | `zsh --no-globalrcs --no-rcs -c "source <root>/<init> && exec zsh --no-globalrcs --no-rcs -i"` |
| `bash` | `bash --noprofile --norc -c "source <root>/<init> && exec bash --noprofile --norc -i"` |
| 其他 | `<program> -c "source <root>/<init> && exec <program> -i"` |

设计文档要求"不直接加载宿主机的全局 shell 配置"，此方案满足。

### 2.3 ls（#3）

当前 `todo!()` panic。

新建 `src/commands/ls.rs`。扫描 `aienv_root()` 目录，过滤含 `manifest.toml` 的子目录，按名称排序输出，每行一个环境名：

```
$ aienv ls
cn
personal
work
```

### 2.4 show（#4）

当前 `todo!()` panic。

新建 `src/commands/show.rs`。加载并解析 manifest，展示解析后的生效配置：

```
Environment:     work
Root:            /Users/panghu/.aienv/work
Inherit CWD:     true
Network:         default

Env Allow:       TERM, LANG, LC_ALL, COLORTERM, PATH
Env Deny:        SSH_AUTH_SOCK, OPENAI_API_KEY, ...
Env Set:         AI_ENV=work

Secrets:         keychain (aienv.work)
Secret Items:    OPENAI_API_KEY, ANTHROPIC_API_KEY

Shell:           /bin/zsh
Shell Init:      env.zsh

Directories:
  HOME             /Users/panghu/.aienv/work/home
  XDG_CONFIG_HOME  /Users/panghu/.aienv/work/config
  XDG_CACHE_HOME   /Users/panghu/.aienv/work/cache
  XDG_DATA_HOME    /Users/panghu/.aienv/work/data
  XDG_STATE_HOME   /Users/panghu/.aienv/work/state
  TMPDIR           /Users/panghu/.aienv/work/tmp
```

不打印 secret 值，只打印 key 名。provider 为 envfile 时显示 `envfile (<root>/secrets.env)`。

---

## 三、Manifest 与解析改进

### 3.1 manifest 段可选化（#10）

`[secrets]`、`[shell]`、`[network]` 改为可选，带 `#[serde(default)]` 和合理默认值：

| 段 | 默认值 |
|----|--------|
| `[secrets]` | `provider = "none"`, `items = []` |
| `[shell]` | `program = "/bin/zsh"`, `init = "env.zsh"` |
| `[network]` | `mode = "default"`, `proxy_url = None` |

最简可用 manifest：

```toml
id = "quick"
root = "/Users/panghu/.aienv/quick"

[env]
allow = ["PATH"]
```

`[env]` 保持必填——环境变量策略是核心语义，用户应显式声明。

### 3.2 secrets provider "none"（#12）

新增 `"none"` 作为合法 provider 值。SecretsConfig 的 Default impl 使用 `provider = "none"`。resolve_secrets 遇到 `"none"` 时直接返回空 map。

### 3.3 envfile 解析升级（#13）

手写解析，不引入外部 crate。规则：

- 空行跳过
- `#` 开头的行视为注释，跳过
- strip `export ` 前缀（含尾部空格）
- `=` 分割后 key 和 value 各 trim 空格
- value 如果被匹配的双引号包裹：去掉引号，处理 `\n`、`\t`、`\\`、`\"` 转义
- value 如果被匹配的单引号包裹：去掉引号，原样保留（无转义）
- 无引号时原样保留
- 不支持多行值
- 不支持无引号值中的行内注释（`KEY=value # comment` 中 `# comment` 作为值的一部分保留）

### 3.4 XDG 目录补全（#17, #18）

init 新增 `data/` 和 `state/` 子目录创建。

`build_child_env` 新增注入：

- `XDG_DATA_HOME` → `<root>/data`
- `XDG_STATE_HOME` → `<root>/state`

强制变量保留列表更新为 7 个：HOME, XDG_CONFIG_HOME, XDG_CACHE_HOME, XDG_DATA_HOME, XDG_STATE_HOME, TMPDIR, AIENV_ROOT。

### 3.5 波浪号展开（#33）

`Manifest::parse()` 返回前，对 `root` 和 `shared_paths` 中以 `~/` 开头的路径做展开，替换为 `$HOME` 的值。只处理 `~/` 前缀，不处理 `~user/` 形式。

### 3.6 init 创建 secrets.env（#14）

`init` 在创建环境目录时，如果 `secrets.env` 不存在，创建空文件并设置权限 600：

```rust
let secrets_path = root.join("secrets.env");
if !secrets_path.exists() {
    fs::write(&secrets_path, "")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&secrets_path, fs::Permissions::from_mode(0o600))?;
    }
}
```

---

## 四、CLI 体验与清理

### 4.1 CLI 改进（#22, #29, #30, #31）

- 添加 `#[command(version)]` 支持 `aienv --version`
- 所有 `--env` 加短形式 `-e`
- 所有参数加 `help` 描述

### 4.2 移除 portable-pty 依赖（#29）

从 Cargo.toml 删除 `portable-pty = "0.8"`。

### 4.3 提取公共路径与加载函数（#19）

新建 `src/env_path.rs`，提供：

```rust
/// 返回 aienv 根目录，优先 AIENV_ROOT，fallback $HOME/.aienv
pub fn aienv_root() -> Result<PathBuf>

/// 返回指定环境的根目录
pub fn env_root(env: &str) -> Result<PathBuf>

/// 加载、校验 manifest（含 id 与目录名一致性校验）
pub fn load_manifest(env: &str) -> Result<(Manifest, PathBuf)>
// 返回 (manifest, env_root_path) 二元组

/// 根据 manifest 解析 secrets
pub fn resolve_secrets(manifest: &Manifest, env_root: &Path) -> Result<BTreeMap<String, String>>
```

从 `exec.rs` 中删除 `load_manifest`。从 `init.rs` 中删除 `env_root`。所有命令共用 `env_path` 模块。

### 4.4 并发执行 run 目录（#26）

每次 exec/shell 启动时：

1. 获取当前进程 PID 作为 exec-id
2. 创建 `<env-root>/run/<pid>/`
3. 注入 `AIENV_EXEC_DIR` 环境变量指向该目录
4. 正常退出时删除该目录（`fs::remove_dir_all`）

使用 `Drop` guard 或在 exec 返回后清理。清理是 best-effort：进程被 SIGKILL 或 `std::process::exit()` 终止时目录会残留，不影响功能。

### 4.5 path_policy 返回 canonicalized paths（#34）

签名改为：

```rust
pub fn validate_cwd(
    cwd: &Path,
    shared_paths: &[PathBuf],
) -> Result<(PathBuf, Vec<PathBuf>)>
```

返回 `(canonicalized_cwd, canonicalized_shared_paths)`。V1 不做路径范围限制，但签名为后续扩展做好准备。

### 4.6 show 输出格式（#20）

见 2.4 节。

### 4.7 测试补全（#32 及相关覆盖）

新增测试：

| 测试 | 验证内容 |
|------|---------|
| deny 优先于 allow | 变量同时在 allow 和 deny 中，验证被排除 |
| env.set 不能覆盖强制变量 | validate 拒绝 `env.set` 含 HOME 等 key |
| inherit_cwd = false | exec 使用 env home 作为 cwd |
| exec 注入 envfile secrets | 端到端：manifest 声明 envfile items，exec 子进程能读到 |
| network.mode = offline | 子进程环境含 http_proxy 指向 127.0.0.1:1 |
| show 输出关键字段 | stdout 包含环境名、目录路径、变量列表 |
| ls 列出多环境 | 创建两个环境后 ls 输出两行 |
| provider "none" | items 为空时 exec 成功 |
| secrets provider 校验 | 非法 provider 字符串被 validate 拒绝 |
| 波浪号展开 | `root = "~/.aienv/test"` 正确展开 |
| manifest id 不匹配 | id 与目录名不同时报错 |
| secrets.env 权限检查 | 权限 644 时拒绝读取 |

---

## build_child_env 完整执行顺序

修复后的最终顺序：

```
1. allow 白名单：从宿主环境继承，deny 列表过滤
2. env.set：manifest 中固定注入的变量
3. secrets：从 keychain/envfile 解析的密钥
4. network.mode：offline/proxy 时注入代理变量
5. 并发目录：注入 AIENV_EXEC_DIR
6. 强制变量（最后，不可覆盖）：
   - HOME → <root>/home
   - XDG_CONFIG_HOME → <root>/config
   - XDG_CACHE_HOME → <root>/cache
   - XDG_DATA_HOME → <root>/data
   - XDG_STATE_HOME → <root>/state
   - TMPDIR → <root>/tmp
   - AIENV_ROOT → <aienv_root>（保留已有值）
```

## init 完整目录结构

```
~/.aienv/<env-id>/
  manifest.toml
  env.zsh
  secrets.env       (新增，权限 600)
  home/
  config/
  cache/
  data/             (新增)
  state/            (新增)
  tmp/
  run/
```

## 文件变更清单

| 操作 | 文件 | 说明 |
|------|------|------|
| 新建 | `src/env_path.rs` | 公共路径解析、manifest 加载、secrets 分发 |
| 新建 | `src/commands/shell.rs` | shell 命令 |
| 新建 | `src/commands/ls.rs` | ls 命令 |
| 新建 | `src/commands/show.rs` | show 命令 |
| 修改 | `src/main.rs` | 合并 tty 分支，接入 shell/ls/show，信号退出码 |
| 修改 | `src/lib.rs` | 导出 env_path 模块 |
| 修改 | `src/cli.rs` | 短形式 -e，help 描述，version |
| 修改 | `src/manifest.rs` | 段可选化，provider/id/保留key 校验，波浪号展开，NetworkConfig.proxy_url |
| 修改 | `src/runtime_env.rs` | 强制变量最后写入，网络模式注入，AIENV_ROOT/AIENV_EXEC_DIR |
| 修改 | `src/secrets.rs` | dotenv 兼容解析，权限检查 |
| 修改 | `src/path_policy.rs` | 返回 canonicalized shared paths |
| 修改 | `src/commands/init.rs` | 补 data/state 目录，创建 secrets.env，补全 deny 列表 |
| 修改 | `src/commands/mod.rs` | 导出 shell、ls、show 模块 |
| 修改 | `src/commands/exec.rs` | 接入 secrets，inherit_cwd，并发 run 目录，使用 env_path |
| 修改 | `src/error.rs` | 不变（anyhow 覆盖新增错误场景） |
| 修改 | `Cargo.toml` | 移除 portable-pty |
| 新建 | `tests/exec_tty.rs` | shell/tty 相关测试 |
| 修改 | `tests/exec_env.rs` | 补充 secrets/deny/inherit_cwd/network 测试 |
| 修改 | `tests/manifest_parsing.rs` | 补充 provider/id/保留key/波浪号测试 |
| 修改 | `tests/init_command.rs` | 验证 data/state/secrets.env 创建 |

## 不做的事

- 多层 manifest 继承（extends）
- 细粒度网络控制（PF 防火墙）
- deny 列表通配符匹配（如 AWS_*）
- 多行 envfile 值
- 非 macOS 平台支持
- 常驻会话管理
- `aienv rm` 删除环境命令
- 审计日志

# CLAUDE.md

## Project

silo — lightweight environment isolation for AI CLI tools on macOS. Creates isolated execution contexts with separate HOME/XDG/credentials per AI identity.

## Tech Stack

- Rust (edition 2024, requires 1.85+)
- clap 4.5 (CLI), serde + toml (manifest), anyhow + thiserror (errors)
- Tests: assert_cmd, predicates, tempfile

## Commands

```bash
cargo build                    # Build
cargo test                     # Run all 50 tests
cargo clippy --all-targets -- -D warnings  # Lint
cargo fmt                      # Format
cargo run -- <subcommand>      # Run locally
```

## Source Structure

```
src/
  main.rs          — CLI entrypoint, dispatches subcommands
  lib.rs           — Module exports
  cli.rs           — clap argument definitions
  manifest.rs      — TOML manifest schema, validation, tilde expansion
  runtime_env.rs   — Builds sanitized child process environment
  env_path.rs      — Shared: silo_root(), env_root(), load_manifest(), resolve_secrets()
  path_policy.rs   — CWD and shared path validation
  secrets.rs       — Dotenv-compatible envfile parser + macOS keychain provider
  error.rs         — AienvError type (manifest parse/validation)
  commands/
    init.rs        — Create environment directory structure
    exec.rs        — Execute command in isolated environment
    shell.rs       — Interactive shell with rc-file suppression
    ls.rs          — List environments
    show.rs        — Display resolved configuration
```

## Key Conventions

- **Forced variables** (HOME, XDG_*, TMPDIR, SILO_ROOT) are written LAST in `build_child_env` — they cannot be overridden by env.set or secrets
- **Manifest sections** `[secrets]`, `[shell]`, `[network]` are optional with defaults
- **Secrets provider** must be `keychain`, `envfile`, or `none`
- **Reserved keys** (HOME, XDG_CONFIG_HOME, XDG_CACHE_HOME, XDG_DATA_HOME, XDG_STATE_HOME, TMPDIR, SILO_ROOT) cannot appear in env.set or secrets.items
- **envfile** uses dotenv format: supports `#` comments, `export` prefix, single/double quotes, escape sequences in double quotes
- **envfile permissions** must be 600 or stricter (checked at read time)
- **Shell rc suppression**: zsh uses `--no-globalrcs --no-rcs`, bash uses `--noprofile --norc`
- **Network offline mode** injects dead proxy `http://127.0.0.1:1`
- **Per-execution run directory** created at `<env-root>/run/<pid>/`, cleaned up on exit (best-effort)
- **SILO_ROOT** preserves existing value for nested `silo exec` calls
- **Manifest id** must match the environment directory name

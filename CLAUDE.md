# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is this

LazyComposer is a TUI (Terminal User Interface) in Rust for managing PHP Composer dependencies, inspired by lazygit. It uses [ratatui](https://ratatui.rs) + [crossterm](https://github.com/crossterm-rs/crossterm).

## Build & Test

```bash
cargo build --release   # Compile binary → ./target/release/lazycomposer
cargo test              # Run all tests
cargo clippy            # Lint (no warnings)
./target/release/lazycomposer [path]   # Run in a directory containing composer.json
```

## Debug logs

Logs write to `~/.local/state/lazycomposer/debug.log`. Monitor with `tail -f ~/.local/state/lazycomposer/debug.log`.

## Architecture

### Package layout

- `src/composer/` — Domain layer (no UI dependency)
  - `types.rs` — Domain types (Package, StringOrBool, AuditResult, Advisory...)
  - `exec.rs` — Executor trait, RealExecutor, streaming
  - `parser.rs` — Parses composer.json + composer.lock, merges into unified packages
  - `runner.rs` — Typed wrapper over composer CLI commands (outdated, audit, require, remove, update)
- `src/config/` — Config resolution, binary validation
- `src/security/` — Package name validation, log sanitization
- `src/logger/` — File logger
- `src/ui/` — TUI layer (ratatui)
  - `app.rs` — Event loop, async loading, key routing
  - `style/` — Color theme + style functions
  - `components/` — TabBar, StatusBar, ConfirmDialog, InputBox, Spinner
  - `panels/` — Packages, Outdated, Audit, Output

### Async data loading

Data loading is asynchronous (background threads + `mpsc` channels) so the UI stays responsive.

### Composer JSON quirks

Composer CLI output has polymorphic fields that need custom deserialization:
- `OutdatedPackage.Abandoned`: `false` (bool) or `"replacement/pkg"` (string) → `StringOrBool`
- `Advisory.CVE` and `Advisory.Severity`: can be `null` → `Option<String>`

### Testing

- `src/composer/` uses mock executors to test Runner without calling the real `composer` binary
- Test fixtures are in `testdata/` (composer.json, composer.lock)

# AGENTS.md — Feedo Development Guide

This file helps AI agents understand and contribute to the Feedo codebase effectively.

## Project Overview

**Feedo** is a terminal RSS reader built with Rust and ratatui. Think Reeder for the command line.


```
feedo/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library root, public API
│   ├── app/             # Application orchestration
│   ├── config/          # Configuration (load/save, structs)
│   ├── feed/            # Feed fetching & parsing
│   ├── opml/            # OPML import/export
│   ├── theme/           # UI theming
│   └── ui/              # Terminal UI (ratatui)
├── Cargo.toml           # Dependencies & metadata
└── README.md            # User documentation
```

## Tech Stack

| Component | Choice | Why |
|-----------|--------|-----|
| Language | Rust 2024 edition | Memory safety, performance |
| Async | Tokio | Non-blocking feed fetching |
| TUI | ratatui + crossterm | Modern, well-maintained |
| RSS Parsing | feed-rs | Handles RSS/Atom/JSON Feed |
| HTTP | reqwest | Async, rustls for TLS |
| Config | serde_json | Human-readable config |
| Errors | color-eyre + thiserror | Rich error context |

## Code Style

### Rust Conventions
- **No `unsafe` code** — `#![forbid(unsafe_code)]`
- **Clippy pedantic** — All lints enabled, fix warnings
- **Documentation** — Doc comments on all public items
- **Error handling** — Use `Result<T>`, propagate with `?`
- **Naming** — `snake_case` functions, `PascalCase` types

### Architecture Patterns
- **Separation of concerns** — Each module has one job
- **Async by default** — Network ops never block UI
- **State is explicit** — No hidden global state
- **Config is XDG-style** — `~/.config/feedo/` on all platforms

### File Organization
```rust
// Module structure pattern:
// mod.rs — public exports, module docs
// data.rs — data structures
// impl files — implementation details
```

## Common Tasks

### Adding a New Feed Source Type
1. Add parser in `src/feed/parser.rs`
2. Update `FeedItem` if new fields needed
3. Add tests for the new format

### Adding a New Keybinding
1. Add handler in `src/ui/input.rs` → `handle_normal_key()`
2. Update status bar hint in `src/ui/render.rs`
3. Document in README.md keybindings table

### Adding a New Theme Color
1. Add variant to `AccentColor` enum in `src/theme/mod.rs`
2. Implement `to_color()` conversion
3. Document in README.md theme section

### Adding a Config Option
1. Add field to `Config` struct in `src/config/data.rs`
2. Add `#[serde(default = "default_fn")]` for backwards compat
3. Document in README.md configuration section

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Check for issues without building
cargo clippy --all-targets

# Format code
cargo fmt
```

## Building

```bash
# Development (fast compile)
cargo build

# Release (optimized, stripped)
cargo build --release

# Run directly
cargo run -- --help
```

## Dependencies Policy

- **Minimize dependencies** — Each dep is a maintenance burden
- **Prefer pure Rust** — No C bindings if avoidable
- **Check maintenance status** — Avoid abandoned crates
- **Pin major versions** — `"1"` not `"*"`

## Git Workflow

- **Commit messages** — Imperative mood, explain *why*
- **One feature per commit** — Atomic, reviewable changes
- **Test before push** — `cargo test && cargo clippy`

## Key Design Decisions

### Why `~/.config/feedo` everywhere?
Cross-platform consistency. Users shouldn't need to remember different paths.

### Why no database?
JSON config is human-editable, version-controllable, and simple. SQLite can come later if needed.

### Why async for feeds?
Fetching 50 feeds sequentially = 50× latency. Async lets us fetch in parallel without blocking the UI.

### Why ratatui over other TUI libs?
Active maintenance, good docs, immediate mode rendering fits our architecture.

## Troubleshooting

### Build fails with openssl errors
We use rustls, not openssl. Check `reqwest` features in Cargo.toml.

### UI looks broken
Ensure terminal supports Unicode. Try: `echo "◕ᴥ◕"`

### Feed not parsing
Check if it's valid RSS/Atom. Some sites serve HTML at feed URLs.

## The website's URL

https://feedo.ricardodantas.me

## Resources

- [ratatui docs](https://docs.rs/ratatui)
- [Tokio tutorial](https://tokio.rs/tokio/tutorial)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [feed-rs examples](https://github.com/feed-rs/feed-rs)

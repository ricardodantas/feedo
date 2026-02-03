# Feedo — Claude Configuration

This directory contains Claude-specific guidance for working with the Feedo codebase.

## Skills

- `rust-tui.md` — Building terminal UIs with ratatui
- `async-rust.md` — Async patterns with Tokio
- `feed-parsing.md` — RSS/Atom parsing strategies

## Quick Reference

### Build & Test
```bash
cargo build              # Dev build
cargo build --release    # Optimized build
cargo test               # Run tests
cargo clippy             # Lint check
```

### Run
```bash
cargo run                # Launch TUI
cargo run -- --help      # Show help
cargo run -- --import x.opml  # Import feeds
```

### Key Files
- `src/app/mod.rs` — Main event loop
- `src/ui/input.rs` — Keyboard handling
- `src/ui/render.rs` — UI rendering
- `src/config/data.rs` — Config structs

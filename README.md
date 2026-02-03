# 🐕 Feedo

```
    ╭─────────────────────────────────────╮
    │                                     │
    │   ┏━╸┏━╸┏━╸╺┳┓┏━┓   🐕              │
    │   ┣╸ ┣╸ ┣╸  ┃┃┃ ┃                   │
    │   ╹  ╹  ┗━╸╺┻┛┗━┛                   │
    │                                     │
    │   Your terminal RSS reader          │
    │                                     │
    ╰─────────────────────────────────────╯
```

A stunning cross-platform terminal RSS reader built with Rust and [ratatui](https://github.com/ratatui/ratatui).

Think [Reeder](https://reederapp.com/) but for your terminal.

## ✨ Features

- 📰 **Beautiful TUI** — Clean, modern three-panel interface
- 📁 **Folders** — Organize feeds into collapsible groups with custom icons
- 🔍 **Search** — Find articles across all feeds instantly
- 🎨 **Themes** — Customizable accent colors
- 📥 **OPML Support** — Import/export your subscriptions
- ⚡ **Fast** — Async feed fetching with Tokio
- 🦀 **Rust** — Memory safe, blazingly fast

## 📦 Installation

### From crates.io (coming soon)

```bash
cargo install feedo
```

### From source

```bash
git clone https://github.com/rdantas/feedo
cd feedo
cargo build --release
./target/release/feedo
```

## 🚀 Usage

```bash
feedo                           # Launch the TUI
feedo --import feeds.opml       # Import feeds from OPML
feedo --export backup.opml      # Export feeds to OPML
feedo --help                    # Show help
```

## ⌨️ Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `l` / `→` / `Enter` | Select / Enter |
| `h` / `←` | Go back |
| `Tab` | Switch panel |
| `/` | Search |
| `r` | Refresh feeds |
| `o` | Open in browser |
| `Space` | Toggle read/unread |
| `a` | Mark all read |
| `g` / `G` | Jump to top/bottom |
| `q` / `Esc` | Quit |

## ⚙️ Configuration

Config location:
- **Linux**: `~/.config/feedo/config.json`
- **macOS**: `~/Library/Application Support/com.feedo.feedo/config.json`
- **Windows**: `%APPDATA%\feedo\feedo\config.json`

### Example config

```json
{
  "folders": [
    {
      "name": "Tech",
      "icon": "💻",
      "expanded": true,
      "feeds": [
        { "name": "Hacker News", "url": "https://hnrss.org/frontpage" },
        { "name": "Lobsters", "url": "https://lobste.rs/rss" }
      ]
    },
    {
      "name": "News",
      "icon": "📰",
      "expanded": false,
      "feeds": [
        { "name": "BBC World", "url": "https://feeds.bbci.co.uk/news/world/rss.xml" }
      ]
    }
  ],
  "feeds": [
    { "name": "xkcd", "url": "https://xkcd.com/rss.xml" }
  ],
  "theme": {
    "accent": "cyan"
  }
}
```

### Theme colors

Available accent colors: `cyan`, `blue`, `green`, `yellow`, `magenta`, `red`, `orange`, `pink`

## 🏗️ Architecture

```
src/
├── main.rs          # Entry point, CLI handling
├── lib.rs           # Library root, module exports
├── app/             # Main application logic
│   └── mod.rs       # App state, event loop
├── config/          # Configuration management
│   ├── mod.rs
│   └── data.rs      # Config data structures
├── feed/            # Feed management
│   ├── mod.rs
│   ├── item.rs      # FeedItem struct
│   ├── manager.rs   # FeedManager, folders
│   └── parser.rs    # RSS/Atom parsing
├── opml/            # OPML import/export
│   └── mod.rs
├── theme/           # Theme configuration
│   └── mod.rs
└── ui/              # Terminal UI
    ├── mod.rs
    ├── state.rs     # UI state
    ├── input.rs     # Key handling
    ├── render.rs    # Rendering logic
    └── widgets/     # Custom widgets
```

## 🗺️ Roadmap

- [ ] Feed discovery (autodiscover RSS from URLs)
- [ ] Offline reading / article caching
- [ ] Keyboard shortcut customization
- [ ] Notifications for new articles
- [ ] Vim-style command mode (`:`)
- [ ] Multiple accounts / sync
- [ ] Smart deduplication

## 📄 License

MIT © Ricardo Dantas

---

*Made with ❤️ and 🦀*

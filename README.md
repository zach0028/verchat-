# VER.CHAT

**The Git for AI conversations.** Import, search, and launch your AI conversations across all your tools — from one terminal.

```
$ verchat
```

![TUI](https://img.shields.io/badge/TUI-interactive-blue) ![Rust](https://img.shields.io/badge/Rust-1.94-orange) ![License](https://img.shields.io/badge/License-Apache_2.0-green)

![VER.CHAT Dashboard](assets/screenshot.png)

## What it does

You use Claude Code, Cursor, LM Studio, Continue.dev, Aider... each stores conversations in its own silo. VER.CHAT brings them all together.

- **Import** conversations from 4 tools (more coming)
- **Search** across everything with full-text search
- **Launch** a conversation into another tool with one keystroke
- **Watch** for new conversations in real-time

## Install

```bash
cargo install --path .
```

## Usage

```bash
verchat              # Launch interactive TUI
verchat init         # Detect AI tools and create config
verchat import --auto # Import all conversations
verchat search "auth" # Search across all tools
verchat log          # List recent conversations
verchat copy <id>    # Copy conversation to clipboard
```

### TUI Shortcuts

| Key | Action |
|-----|--------|
| `/` | Search |
| `⏎` | Open conversation |
| `c` | Copy to clipboard |
| `l` | Launch in another tool |
| `s` | Stats |
| `q` | Quit |

## Supported tools

| Tool | Format | Status |
|------|--------|--------|
| Claude Code | JSONL | ✅ |
| LM Studio | JSON | ✅ |
| Continue.dev | JSON | ✅ |
| Aider | Markdown | ✅ |
| Cursor | SQLite | Planned |
| Windsurf | Protobuf | Planned |

## How it works

```
Claude Code ──┐
LM Studio   ──┼──► SQLite (local) ──► TUI / CLI
Continue.dev ──┤
Aider       ──┘
```

All data stays on your machine. No cloud, no account, no network.

## License

Apache 2.0

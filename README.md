# VER.CHAT

**The Git for AI conversations.** Import, search, and launch your AI conversations across all your tools — from one terminal.

```
$ verchat
```

![TUI](https://img.shields.io/badge/TUI-interactive-blue) ![Rust](https://img.shields.io/badge/Rust-1.94-orange) ![License](https://img.shields.io/badge/License-Apache_2.0-green)

![VER.CHAT Dashboard](assets/screenshot.png)

## What it does

You use Claude Code, Cursor, LM Studio, Continue.dev, Aider... each stores conversations in its own silo. VER.CHAT brings them all together.

- **Import** conversations from 6 AI tools
- **Search** across everything with full-text search
- **Launch** a conversation into another tool with one keystroke
- **Watch** for new conversations in real-time
- **Track** token usage (input, cache, output) per conversation

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
| Gemini CLI | JSON | ✅ |
| OpenCode | SQLite | ✅ |
| Cursor | SQLite + Protobuf | ✅ |
| Aider | Markdown | ✅ |
| Windsurf | Protobuf | Experimental |

## How it works

```
Claude Code  ──┐
LM Studio    ──┤
Continue.dev ──┤
Gemini CLI   ──┼──► SQLite (local) ──► TUI / CLI
OpenCode     ──┤
Cursor       ──┤
Aider        ──┘
```

All data stays on your machine. No cloud, no account, no network.

## License

Apache 2.0

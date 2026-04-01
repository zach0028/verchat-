# VER.CHAT

**The Git for AI conversations.** Import, search, and launch your AI conversations across all your tools — from one terminal.

```
$ verchat
```

![TUI](https://img.shields.io/badge/TUI-interactive-blue) ![Rust](https://img.shields.io/badge/Rust-1.94-orange) ![License](https://img.shields.io/badge/License-Apache_2.0-green)

![VER.CHAT Dashboard](assets/screenshot.png)

## What it does

You use Claude Code, Cursor, LM Studio, Continue.dev, Gemini CLI... each stores conversations in its own silo. VER.CHAT brings them all together.

- **Import** conversations from 7 AI tools
- **Search** across everything with full-text search
- **Launch** a conversation into another tool — with smart compression to fit any context window
- **Watch** for new conversations in real-time
- **Track** token usage (input, cache write, cache read, output) per conversation

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
verchat source list  # Show configured sources
verchat source add <tool> <path>  # Add a custom path
```

### TUI Shortcuts

| Key | Action |
|-----|--------|
| `/` | Search |
| `⏎` | Open conversation |
| `c` | Copy to clipboard (Markdown) |
| `l` | Launch in another tool |
| `s` | Stats |
| `↑↓` or `j/k` | Navigate |
| Scroll wheel | Navigate / scroll |
| `q` | Quit |

### Smart Launch

When launching a conversation into another tool, VER.CHAT:

1. Analyzes the conversation (dialogue tokens vs tool calls / thinking)
2. Asks for your target context window (presets: 8K, 32K, 64K, 128K, 256K, 1M)
3. Compresses if needed (keeps beginning + end, removes middle)
4. Injects natively (LM Studio, Continue.dev) or copies to clipboard

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

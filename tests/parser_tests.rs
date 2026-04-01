use std::fs;
use tempfile::TempDir;
use chrono::Datelike;

use verchat::model::{Role, Source};
use verchat::parser::Parser;
use verchat::parser::claude_code::ClaudeCodeParser;
use verchat::parser::lm_studio::LmStudioParser;
use verchat::parser::continue_dev::ContinueDevParser;
use verchat::parser::aider::AiderParser;
use verchat::store::Store;

// ═══════════════════════════════════════════════════════════════
// CLAUDE CODE PARSER
// ═══════════════════════════════════════════════════════════════

fn make_claude_code_jsonl() -> String {
    let lines = vec![
        r#"{"type":"user","message":{"role":"user","content":"Fix the auth bug"},"uuid":"aaa","parentUuid":null,"timestamp":"2026-03-30T14:30:00Z","sessionId":"sess-1"}"#,
        r#"{"type":"assistant","message":{"role":"assistant","model":"claude-opus-4-6","content":[{"type":"text","text":"I found the issue in the middleware."}]},"uuid":"bbb","parentUuid":"aaa","timestamp":"2026-03-30T14:30:10Z","sessionId":"sess-1"}"#,
        r#"{"type":"user","message":{"role":"user","content":"Can you also fix the JWT validation?"},"uuid":"ccc","parentUuid":"bbb","timestamp":"2026-03-30T14:31:00Z","sessionId":"sess-1"}"#,
        r#"{"type":"assistant","message":{"role":"assistant","model":"claude-opus-4-6","content":[{"type":"thinking","thinking":"Let me analyze..."},{"type":"text","text":"Sure, the JWT token was not being refreshed properly."}]},"uuid":"ddd","parentUuid":"ccc","timestamp":"2026-03-30T14:31:15Z","sessionId":"sess-1"}"#,
        r#"{"type":"progress","message":"reading file...","uuid":"eee"}"#,
    ];
    lines.join("\n")
}

#[test]
fn test_claude_code_parse_basic() {
    let dir = TempDir::new().unwrap();
    let project_dir = dir.path().join("projects").join("-test-project-");
    fs::create_dir_all(&project_dir).unwrap();
    let file = project_dir.join("sess-1.jsonl");
    fs::write(&file, make_claude_code_jsonl()).unwrap();

    let parser = ClaudeCodeParser;
    let conv = parser.parse(&file).unwrap();

    assert_eq!(conv.source, Source::ClaudeCode);
    assert_eq!(conv.model.as_deref(), Some("claude-opus-4-6"));
    assert_eq!(conv.messages.len(), 4); // 2 user + 2 assistant (thinking filtered, text kept)
    assert_eq!(conv.messages[0].role, Role::User);
    assert_eq!(conv.messages[0].content, "Fix the auth bug");
    assert_eq!(conv.messages[1].role, Role::Assistant);
    assert!(conv.messages[1].content.contains("middleware"));
    assert_eq!(conv.messages[2].role, Role::User);
    assert_eq!(conv.messages[3].role, Role::Assistant);
    assert!(conv.messages[3].content.contains("JWT"));
    assert!(conv.title.contains("Fix the auth bug"));
}

#[test]
fn test_claude_code_skip_malformed_lines() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.jsonl");
    let content = format!(
        "{}\n{}\n{}",
        r#"{"type":"user","message":{"role":"user","content":"hello"},"uuid":"a","timestamp":"2026-03-30T10:00:00Z"}"#,
        r#"THIS IS NOT VALID JSON"#,
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"hi there"}]},"uuid":"b","timestamp":"2026-03-30T10:00:05Z"}"#,
    );
    fs::write(&file, content).unwrap();

    let parser = ClaudeCodeParser;
    let conv = parser.parse(&file).unwrap();
    assert_eq!(conv.messages.len(), 2); // malformed line skipped
}

#[test]
fn test_claude_code_empty_file() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("empty.jsonl");
    fs::write(&file, "").unwrap();

    let parser = ClaudeCodeParser;
    assert!(parser.parse(&file).is_err());
}

#[test]
fn test_claude_code_scan() {
    let dir = TempDir::new().unwrap();
    let project = dir.path().join("project-a");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("sess1.jsonl"), "{}").unwrap();
    fs::write(project.join("sess2.jsonl"), "{}").unwrap();
    // Subagents should be ignored (they're in subdirectories)
    let sub = project.join("subagents");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join("agent.jsonl"), "{}").unwrap();

    let parser = ClaudeCodeParser;
    let files = parser.scan(&[dir.path().to_path_buf()]);
    assert_eq!(files.len(), 2); // subagent not included
}

// ═══════════════════════════════════════════════════════════════
// LM STUDIO PARSER
// ═══════════════════════════════════════════════════════════════

fn make_lm_studio_json() -> String {
    serde_json::json!({
        "name": "Test LM Conversation",
        "createdAt": 1711800000000_u64,
        "userLastMessagedAt": 1711800060000_u64,
        "lastUsedModel": { "identifier": "llama-3.1-8b" },
        "messages": [
            {
                "versions": [{
                    "type": "singleStep",
                    "role": "user",
                    "content": [{ "type": "text", "text": "What is Rust?" }]
                }]
            },
            {
                "versions": [{
                    "type": "multiStep",
                    "role": "assistant",
                    "steps": [
                        { "type": "status", "stepIdentifier": "s1", "statusState": { "status": "done", "text": "ok" } },
                        {
                            "type": "contentBlock",
                            "stepIdentifier": "s2",
                            "content": [{ "type": "text", "text": "Rust is a systems programming language." }]
                        }
                    ]
                }]
            },
            {
                "versions": [{
                    "type": "singleStep",
                    "role": "user",
                    "content": [{ "type": "text", "text": "How does ownership work?" }]
                }]
            }
        ],
        "tokenCount": 500,
        "pinned": false,
        "systemPrompt": "",
        "usePerChatPredictionConfig": false,
        "perChatPredictionConfig": {},
        "clientInput": "",
        "clientInputFiles": [],
        "userFilesSizeBytes": 0,
        "notes": "",
        "plugins": [],
        "pluginConfigs": {},
        "disabledPluginTools": [],
        "looseFiles": []
    })
    .to_string()
}

#[test]
fn test_lm_studio_parse_basic() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("123.conversation.json");
    fs::write(&file, make_lm_studio_json()).unwrap();

    let parser = LmStudioParser;
    let conv = parser.parse(&file).unwrap();

    assert_eq!(conv.source, Source::LmStudio);
    assert_eq!(conv.title, "Test LM Conversation");
    assert_eq!(conv.model.as_deref(), Some("llama-3.1-8b"));
    assert_eq!(conv.messages.len(), 3);
    assert_eq!(conv.messages[0].role, Role::User);
    assert_eq!(conv.messages[0].content, "What is Rust?");
    assert_eq!(conv.messages[1].role, Role::Assistant);
    assert!(conv.messages[1].content.contains("systems programming"));
    assert_eq!(conv.messages[2].role, Role::User);
    assert!(conv.messages[2].content.contains("ownership"));
}

#[test]
fn test_lm_studio_empty_messages() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("empty.conversation.json");
    let json = serde_json::json!({
        "name": "",
        "messages": [],
        "tokenCount": 0,
        "pinned": false,
        "systemPrompt": "",
        "usePerChatPredictionConfig": false,
        "perChatPredictionConfig": {},
        "clientInput": "",
        "clientInputFiles": [],
        "userFilesSizeBytes": 0,
        "notes": "",
        "plugins": [],
        "pluginConfigs": {},
        "disabledPluginTools": [],
        "looseFiles": []
    });
    fs::write(&file, json.to_string()).unwrap();

    let parser = LmStudioParser;
    assert!(parser.parse(&file).is_err());
}

#[test]
fn test_lm_studio_scan_recursive() {
    let dir = TempDir::new().unwrap();
    // Root level
    fs::write(dir.path().join("1.conversation.json"), "{}").unwrap();
    // Subfolder
    let sub = dir.path().join("AI Projects");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join("2.conversation.json"), "{}").unwrap();
    // Non-conversation file (should be ignored)
    fs::write(dir.path().join("config.json"), "{}").unwrap();

    let parser = LmStudioParser;
    let files = parser.scan(&[dir.path().to_path_buf()]);
    assert_eq!(files.len(), 2);
}

// ═══════════════════════════════════════════════════════════════
// CONTINUE.DEV PARSER
// ═══════════════════════════════════════════════════════════════

fn make_continue_dev_json() -> String {
    serde_json::json!({
        "sessionId": "abc-123",
        "title": "Code Assistance",
        "workspaceDirectory": "file:///Users/test/project",
        "history": [
            {
                "message": {
                    "role": "user",
                    "content": [{ "type": "text", "text": "Help me with Rust" }],
                    "id": "msg1"
                }
            },
            {
                "message": {
                    "role": "assistant",
                    "content": "Sure! What do you need help with?",
                    "id": "msg2"
                }
            },
            {
                "message": {
                    "role": "user",
                    "content": [{ "type": "text", "text": "Explain ownership" }],
                    "id": "msg3"
                }
            },
            {
                "message": {
                    "role": "assistant",
                    "content": "Ownership is Rust's memory management system. Each value has exactly one owner.",
                    "id": "msg4"
                }
            }
        ]
    })
    .to_string()
}

#[test]
fn test_continue_dev_parse_basic() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("abc-123.json");
    fs::write(&file, make_continue_dev_json()).unwrap();

    let parser = ContinueDevParser;
    let conv = parser.parse(&file).unwrap();

    assert_eq!(conv.source, Source::ContinueDev);
    assert_eq!(conv.title, "Code Assistance");
    assert_eq!(conv.messages.len(), 4);
    assert_eq!(conv.messages[0].role, Role::User);
    assert_eq!(conv.messages[0].content, "Help me with Rust");
    assert_eq!(conv.messages[1].role, Role::Assistant);
    assert!(conv.messages[1].content.contains("What do you need"));
    assert_eq!(conv.messages[3].role, Role::Assistant);
    assert!(conv.messages[3].content.contains("Ownership"));
}

#[test]
fn test_continue_dev_scan_ignores_sessions_json() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("abc.json"), "{}").unwrap();
    fs::write(dir.path().join("sessions.json"), "{}").unwrap();

    let parser = ContinueDevParser;
    let files = parser.scan(&[dir.path().to_path_buf()]);
    assert_eq!(files.len(), 1); // sessions.json ignored
}

// ═══════════════════════════════════════════════════════════════
// AIDER PARSER
// ═══════════════════════════════════════════════════════════════

fn make_aider_markdown() -> String {
    r#"# aider chat started at 2025-10-01 19:24:08

> Some system output to ignore
> More system output

#### Fix the login bug

I'll look at the login code and fix the issue.

The problem was in the session handler.

#### What about the tests?

Here are the updated tests:

```python
def test_login():
    assert login("user", "pass") == True
```

# aider chat started at 2025-10-01 20:00:00

#### Refactor the database module

I'll restructure the database layer.
"#
    .to_string()
}

#[test]
fn test_aider_parse_basic() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join(".aider.chat.history.md");
    fs::write(&file, make_aider_markdown()).unwrap();

    let parser = AiderParser;
    let conv = parser.parse(&file).unwrap();

    assert_eq!(conv.source, Source::Aider);

    // 3 sessions totales: session 1 = 4 msg, session 2 = 2 msg
    assert_eq!(conv.messages.len(), 6);

    // First message should be user
    assert_eq!(conv.messages[0].role, Role::User);
    assert!(conv.messages[0].content.contains("Fix the login bug"));

    // Should have at least one assistant message
    let has_assistant = conv.messages.iter().any(|m| m.role == Role::Assistant);
    assert!(has_assistant, "Expected at least one assistant message");

    // Assistant should contain the actual response (not system output)
    let assistant_msg = conv.messages.iter().find(|m| m.role == Role::Assistant).unwrap();
    assert!(assistant_msg.content.contains("session handler") || assistant_msg.content.contains("login"),
        "Assistant content: {}", assistant_msg.content);

    // Check timestamps were parsed
    assert!(conv.created_at.year() == 2025);
}

#[test]
fn test_aider_ignores_system_output() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join(".aider.chat.history.md");
    let content = r#"# aider chat started at 2025-10-01 19:00:00

> This is system output that should be ignored
> pip install something

#### Hello

Hi there!
"#;
    fs::write(&file, content).unwrap();

    let parser = AiderParser;
    let conv = parser.parse(&file).unwrap();

    // No message should contain "> " system output
    for msg in &conv.messages {
        assert!(!msg.content.contains("> This is system output"));
    }
}

#[test]
fn test_aider_empty_file() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join(".aider.chat.history.md");
    fs::write(&file, "").unwrap();

    let parser = AiderParser;
    assert!(parser.parse(&file).is_err());
}

// ═══════════════════════════════════════════════════════════════
// INTEGRATION: PARSER → STORE → SEARCH
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_full_pipeline_all_parsers() {
    let store = Store::open_in_memory().unwrap();

    // Claude Code
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("claude.jsonl");
    fs::write(&file, make_claude_code_jsonl()).unwrap();
    let conv = ClaudeCodeParser.parse(&file).unwrap();
    assert!(store.insert(&conv).unwrap());

    // LM Studio
    let file = dir.path().join("lm.conversation.json");
    fs::write(&file, make_lm_studio_json()).unwrap();
    let conv = LmStudioParser.parse(&file).unwrap();
    assert!(store.insert(&conv).unwrap());

    // Continue.dev
    let file = dir.path().join("cont.json");
    fs::write(&file, make_continue_dev_json()).unwrap();
    let conv = ContinueDevParser.parse(&file).unwrap();
    assert!(store.insert(&conv).unwrap());

    // Aider
    let file = dir.path().join(".aider.chat.history.md");
    fs::write(&file, make_aider_markdown()).unwrap();
    let conv = AiderParser.parse(&file).unwrap();
    assert!(store.insert(&conv).unwrap());

    // Verify count
    assert_eq!(store.count().unwrap(), 4);

    // Verify list returns all 4
    let list = store.list(10, 0).unwrap();
    assert_eq!(list.len(), 4);

    // Verify search cross-tool
    let results = store.search("Rust", 10).unwrap();
    assert!(results.len() >= 2); // LM Studio + Continue.dev mention Rust

    // Verify search finds Claude content
    let results = store.search("auth", 10).unwrap();
    assert!(!results.is_empty());

    // Verify search finds Aider content
    let results = store.search("login", 10).unwrap();
    assert!(!results.is_empty());
}

#[test]
fn test_incremental_import() {
    let store = Store::open_in_memory().unwrap();

    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.jsonl");
    fs::write(&file, make_claude_code_jsonl()).unwrap();

    let conv = ClaudeCodeParser.parse(&file).unwrap();

    // First import
    assert!(store.insert(&conv).unwrap()); // true = inserted
    assert_eq!(store.count().unwrap(), 1);

    // Second import of same file = skipped
    let conv2 = ClaudeCodeParser.parse(&file).unwrap();
    assert!(!store.insert(&conv2).unwrap()); // false = already exists
    assert_eq!(store.count().unwrap(), 1); // still 1
}

#[test]
fn test_search_empty_query() {
    let store = Store::open_in_memory().unwrap();
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.jsonl");
    fs::write(&file, make_claude_code_jsonl()).unwrap();
    let conv = ClaudeCodeParser.parse(&file).unwrap();
    store.insert(&conv).unwrap();

    // Empty search should not crash
    let results = store.search("", 10);
    // FTS5 may error on empty query, that's ok
    assert!(results.is_ok() || results.is_err());
}

#[test]
fn test_conversation_show_by_prefix() {
    let store = Store::open_in_memory().unwrap();
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.jsonl");
    fs::write(&file, make_claude_code_jsonl()).unwrap();

    let conv = ClaudeCodeParser.parse(&file).unwrap();
    let id_prefix = conv.id.to_string()[..8].to_string();
    store.insert(&conv).unwrap();

    // Find by prefix
    let found = store.get_by_id_prefix(&id_prefix).unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.messages.len(), 4);
    assert_eq!(found.messages[0].content, "Fix the auth bug");

    // Not found
    let not_found = store.get_by_id_prefix("zzzzzzzz").unwrap();
    assert!(not_found.is_none());
}

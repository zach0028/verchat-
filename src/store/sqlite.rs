use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use uuid::Uuid;

use crate::model::{Conversation, Message, Role, Source};

/// Erreurs du store.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("Unknown source: {0}")]
    UnknownSource(String),
}

/// Résultat léger d'une conversation pour les listes (sans les messages).
#[derive(Debug)]
pub struct ConversationSummary {
    pub id: Uuid,
    pub title: String,
    pub source: Source,
    pub model: Option<String>,
    pub source_path: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub favorite: bool,
    pub message_count: usize,
    pub tokens_input: u64,
    pub tokens_cache_write: u64,
    pub tokens_cache_read: u64,
    pub tokens_output: u64,
}

/// Résultat d'une recherche full-text.
#[derive(Debug)]
pub struct SearchResult {
    pub conversation: ConversationSummary,
    pub snippet: String,
}

/// Store SQLite — couche de persistance locale.
///
/// Un seul fichier `store.db` contient toutes les conversations,
/// messages, et l'index full-text (FTS5).
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Ouvre (ou crée) la base de données au chemin donné.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;

        // Performance : WAL mode + foreign keys
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA synchronous = NORMAL;",
        )?;

        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    /// Crée une base en mémoire (pour les tests).
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    /// Crée les tables et index si ils n'existent pas encore.
    fn migrate(&self) -> Result<(), StoreError> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS conversations (
                id          TEXT PRIMARY KEY,
                title       TEXT NOT NULL,
                source      TEXT NOT NULL,
                model       TEXT,
                source_path TEXT NOT NULL UNIQUE,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL,
                favorite    INTEGER NOT NULL DEFAULT 0,
                tokens_input       INTEGER NOT NULL DEFAULT 0,
                tokens_cache_write INTEGER NOT NULL DEFAULT 0,
                tokens_cache_read  INTEGER NOT NULL DEFAULT 0,
                tokens_output      INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS messages (
                id              TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
                role            TEXT NOT NULL,
                content         TEXT NOT NULL,
                timestamp       TEXT,
                position        INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_messages_conversation
                ON messages(conversation_id);

            -- Index full-text sur le contenu des messages
            CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
                content,
                conversation_id UNINDEXED,
                content=messages,
                content_rowid=rowid
            );

            -- Triggers pour garder l'index FTS synchronisé
            CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
                INSERT INTO messages_fts(rowid, content, conversation_id)
                VALUES (new.rowid, new.content, new.conversation_id);
            END;

            CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
                INSERT INTO messages_fts(messages_fts, rowid, content, conversation_id)
                VALUES ('delete', old.rowid, old.content, old.conversation_id);
            END;

            CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages BEGIN
                INSERT INTO messages_fts(messages_fts, rowid, content, conversation_id)
                VALUES ('delete', old.rowid, old.content, old.conversation_id);
                INSERT INTO messages_fts(rowid, content, conversation_id)
                VALUES (new.rowid, new.content, new.conversation_id);
            END;",
        )?;

        // Migration : ajouter les colonnes tokens si absentes (DB existante)
        let has_tokens: bool = self.conn
            .prepare("SELECT tokens_input FROM conversations LIMIT 0")
            .is_ok();
        if !has_tokens {
            self.conn.execute_batch(
                "ALTER TABLE conversations ADD COLUMN tokens_input INTEGER NOT NULL DEFAULT 0;
                 ALTER TABLE conversations ADD COLUMN tokens_cache_write INTEGER NOT NULL DEFAULT 0;
                 ALTER TABLE conversations ADD COLUMN tokens_cache_read INTEGER NOT NULL DEFAULT 0;
                 ALTER TABLE conversations ADD COLUMN tokens_output INTEGER NOT NULL DEFAULT 0;",
            )?;
        }

        Ok(())
    }

    /// Insère une conversation complète (avec ses messages).
    /// Si le `source_path` existe déjà, la conversation est ignorée (import incrémental).
    pub fn insert(&self, conv: &Conversation) -> Result<bool, StoreError> {
        // Vérifier si déjà importée (par source_path)
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM conversations WHERE source_path = ?1)",
            params![conv.source_path],
            |row| row.get(0),
        )?;

        if exists {
            return Ok(false);
        }

        let tx = self.conn.unchecked_transaction()?;

        tx.execute(
            "INSERT INTO conversations (id, title, source, model, source_path, created_at, updated_at, favorite, tokens_input, tokens_cache_write, tokens_cache_read, tokens_output)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                conv.id.to_string(),
                conv.title,
                source_to_str(&conv.source),
                conv.model,
                conv.source_path,
                conv.created_at.to_rfc3339(),
                conv.updated_at.to_rfc3339(),
                conv.favorite as i32,
                conv.tokens_input as i64,
                conv.tokens_cache_write as i64,
                conv.tokens_cache_read as i64,
                conv.tokens_output as i64,
            ],
        )?;

        for (position, msg) in conv.messages.iter().enumerate() {
            tx.execute(
                "INSERT INTO messages (id, conversation_id, role, content, timestamp, position)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    msg.id.to_string(),
                    conv.id.to_string(),
                    role_to_str(&msg.role),
                    msg.content,
                    msg.timestamp.map(|ts| ts.to_rfc3339()),
                    position as i64,
                ],
            )?;
        }

        tx.commit()?;
        Ok(true)
    }

    /// Liste les conversations, les plus récentes en premier.
    pub fn list(&self, limit: usize, offset: usize) -> Result<Vec<ConversationSummary>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.title, c.source, c.model, c.source_path,
                    c.created_at, c.updated_at, c.favorite,
                    COUNT(m.id) as message_count,
                    c.tokens_input, c.tokens_cache_write, c.tokens_cache_read, c.tokens_output
             FROM conversations c
             LEFT JOIN messages m ON m.conversation_id = c.id
             GROUP BY c.id
             ORDER BY c.updated_at DESC
             LIMIT ?1 OFFSET ?2",
        )?;

        let rows = stmt.query_map(params![limit as i64, offset as i64], |row| {
            Ok(ConversationSummary {
                id: row.get::<_, String>(0).map(|s| Uuid::parse_str(&s).unwrap_or_default())?,
                title: row.get(1)?,
                source: str_to_source(&row.get::<_, String>(2)?),
                model: row.get(3)?,
                source_path: row.get(4)?,
                created_at: parse_dt(&row.get::<_, String>(5)?),
                updated_at: parse_dt(&row.get::<_, String>(6)?),
                favorite: row.get::<_, i32>(7)? != 0,
                message_count: row.get::<_, i64>(8)? as usize,
                tokens_input: row.get::<_, i64>(9).unwrap_or(0) as u64,
                tokens_cache_write: row.get::<_, i64>(10).unwrap_or(0) as u64,
                tokens_cache_read: row.get::<_, i64>(11).unwrap_or(0) as u64,
                tokens_output: row.get::<_, i64>(12).unwrap_or(0) as u64,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(StoreError::from)
    }

    /// Recherche full-text dans les messages. Retourne les conversations qui matchent
    /// avec un extrait du message le plus pertinent.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, StoreError> {
        // FTS5 snippet() ne supporte pas GROUP BY.
        // On sélectionne le meilleur match par conversation via une sous-requête.
        let mut stmt = self.conn.prepare(
            "WITH matched AS (
                SELECT m.conversation_id,
                       m.content,
                       rank
                FROM messages_fts
                JOIN messages m ON m.rowid = messages_fts.rowid
                WHERE messages_fts MATCH ?1
            ),
            best_per_conv AS (
                SELECT conversation_id,
                       content as snippet,
                       MIN(rank) as best_rank
                FROM matched
                GROUP BY conversation_id
            )
            SELECT c.id, c.title, c.source, c.model, c.source_path,
                   c.created_at, c.updated_at, c.favorite,
                   (SELECT COUNT(*) FROM messages WHERE conversation_id = c.id) as message_count,
                   SUBSTR(b.snippet, 1, 200) as snippet,
                   c.tokens_input, c.tokens_cache_write, c.tokens_cache_read, c.tokens_output
            FROM best_per_conv b
            JOIN conversations c ON c.id = b.conversation_id
            ORDER BY b.best_rank
            LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![query, limit as i64], |row| {
            Ok(SearchResult {
                conversation: ConversationSummary {
                    id: row.get::<_, String>(0).map(|s| Uuid::parse_str(&s).unwrap_or_default())?,
                    title: row.get(1)?,
                    source: str_to_source(&row.get::<_, String>(2)?),
                    model: row.get(3)?,
                    source_path: row.get(4)?,
                    created_at: parse_dt(&row.get::<_, String>(5)?),
                    updated_at: parse_dt(&row.get::<_, String>(6)?),
                    favorite: row.get::<_, i32>(7)? != 0,
                    message_count: row.get::<_, i64>(8)? as usize,
                    tokens_input: row.get::<_, i64>(10).unwrap_or(0) as u64,
                    tokens_cache_write: row.get::<_, i64>(11).unwrap_or(0) as u64,
                    tokens_cache_read: row.get::<_, i64>(12).unwrap_or(0) as u64,
                    tokens_output: row.get::<_, i64>(13).unwrap_or(0) as u64,
                },
                snippet: row.get(9)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(StoreError::from)
    }

    /// Récupère une conversation complète (avec messages) par préfixe d'ID.
    /// Permet de taper juste les premiers caractères de l'UUID.
    pub fn get_by_id_prefix(&self, prefix: &str) -> Result<Option<Conversation>, StoreError> {
        let pattern = format!("{prefix}%");

        let row = self.conn.query_row(
            "SELECT id, title, source, model, source_path, created_at, updated_at, favorite, tokens_input, tokens_cache_write, tokens_cache_read, tokens_output
             FROM conversations WHERE id LIKE ?1 LIMIT 1",
            params![pattern],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i32>(7)?,
                    row.get::<_, i64>(8).unwrap_or(0),
                    row.get::<_, i64>(9).unwrap_or(0),
                    row.get::<_, i64>(10).unwrap_or(0),
                    row.get::<_, i64>(11).unwrap_or(0),
                ))
            },
        );

        let (id_str, title, source_str, model, source_path, created_str, updated_str, fav, tk_input, tk_cw, tk_cr, tk_out) =
            match row {
                Ok(r) => r,
                Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
                Err(e) => return Err(StoreError::from(e)),
            };

        // Charger les messages
        let mut stmt = self.conn.prepare(
            "SELECT id, role, content, timestamp
             FROM messages
             WHERE conversation_id = ?1
             ORDER BY position ASC",
        )?;

        let messages = stmt
            .query_map(params![id_str], |row| {
                Ok(Message {
                    id: row.get::<_, String>(0)
                        .map(|s| Uuid::parse_str(&s).unwrap_or_default())?,
                    role: str_to_role(&row.get::<_, String>(1)?),
                    content: row.get(2)?,
                    timestamp: row.get::<_, Option<String>>(3)?
                        .map(|s| parse_dt(&s)),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Some(Conversation {
            id: Uuid::parse_str(&id_str).unwrap_or_default(),
            title,
            source: str_to_source(&source_str),
            model,
            source_path,
            created_at: parse_dt(&created_str),
            updated_at: parse_dt(&updated_str),
            favorite: fav != 0,
            tags: Vec::new(),
            tokens_input: tk_input as u64,
            tokens_cache_write: tk_cw as u64,
            tokens_cache_read: tk_cr as u64,
            tokens_output: tk_out as u64,
            messages,
        }))
    }

    /// Nombre total de conversations dans le store.
    pub fn count(&self) -> Result<usize, StoreError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM conversations",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }
}

// --- Helpers de conversion ---

fn source_to_str(source: &Source) -> &'static str {
    match source {
        Source::ClaudeCode => "claude-code",
        Source::LmStudio => "lm-studio",
        Source::ContinueDev => "continue-dev",
        Source::Aider => "aider",
    }
}

fn str_to_source(s: &str) -> Source {
    match s {
        "claude-code" => Source::ClaudeCode,
        "lm-studio" => Source::LmStudio,
        "continue-dev" => Source::ContinueDev,
        "aider" => Source::Aider,
        _ => Source::ClaudeCode, // fallback safe
    }
}

fn role_to_str(role: &Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::System => "system",
    }
}

fn str_to_role(s: &str) -> Role {
    match s {
        "user" => Role::User,
        "assistant" => Role::Assistant,
        "system" => Role::System,
        _ => Role::User,
    }
}

fn parse_dt(s: &str) -> DateTime<Utc> {
    s.parse::<DateTime<Utc>>().unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Conversation, Message, Role, Source};

    fn make_test_conversation() -> Conversation {
        Conversation::new(
            "Test conversation".to_string(),
            Source::ClaudeCode,
            Some("claude-opus-4-6".to_string()),
            "/tmp/test.jsonl".to_string(),
            Utc::now(),
            Utc::now(),
            vec![
                Message::new(Role::User, "Fix the auth bug".to_string(), Some(Utc::now())),
                Message::new(Role::Assistant, "I found the issue in the middleware".to_string(), Some(Utc::now())),
            ],
        )
    }

    #[test]
    fn test_insert_and_list() {
        let store = Store::open_in_memory().unwrap();
        let conv = make_test_conversation();

        let inserted = store.insert(&conv).unwrap();
        assert!(inserted);

        // Deuxième insert = ignoré (même source_path)
        let inserted_again = store.insert(&conv).unwrap();
        assert!(!inserted_again);

        let list = store.list(10, 0).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "Test conversation");
        assert_eq!(list[0].message_count, 2);
    }

    #[test]
    fn test_search() {
        let store = Store::open_in_memory().unwrap();
        let conv = make_test_conversation();
        store.insert(&conv).unwrap();

        let results = store.search("auth bug", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].snippet.contains("auth"));

        let no_results = store.search("nonexistent query", 10).unwrap();
        assert!(no_results.is_empty());
    }

    #[test]
    fn test_count() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(store.count().unwrap(), 0);

        store.insert(&make_test_conversation()).unwrap();
        assert_eq!(store.count().unwrap(), 1);
    }
}

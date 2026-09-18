CREATE TABLE IF NOT EXISTS memory_entries (
    id TEXT PRIMARY KEY NOT NULL,
    character_id TEXT NOT NULL,
    chat_id TEXT,
    scope TEXT NOT NULL CHECK (scope IN ('chat', 'character')),
    content TEXT NOT NULL,
    content_normalized TEXT NOT NULL,
    importance INTEGER NOT NULL CHECK (importance BETWEEN 1 AND 100),
    manually_created INTEGER NOT NULL DEFAULT 0 CHECK (manually_created IN (0, 1)),
    source_start_message_id TEXT,
    source_end_message_id TEXT,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE,
    FOREIGN KEY (chat_id) REFERENCES chats(id) ON DELETE CASCADE,
    CHECK ((scope = 'character' AND chat_id IS NULL) OR (scope = 'chat' AND chat_id IS NOT NULL))
);

CREATE INDEX IF NOT EXISTS idx_memory_entries_scope
    ON memory_entries(character_id, scope, chat_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_memory_entries_normalized
    ON memory_entries(character_id, scope, chat_id, content_normalized);

CREATE TABLE IF NOT EXISTS memory_vectors (
    memory_id TEXT PRIMARY KEY NOT NULL,
    embedding_model TEXT NOT NULL,
    embedding_dimension INTEGER NOT NULL CHECK (embedding_dimension > 0),
    vector_blob BLOB NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY (memory_id) REFERENCES memory_entries(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_memory_vectors_model
    ON memory_vectors(embedding_model, embedding_dimension);

CREATE TABLE IF NOT EXISTS memory_chat_state (
    chat_id TEXT PRIMARY KEY NOT NULL,
    last_processed_message_id TEXT,
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY (chat_id) REFERENCES chats(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS message_memory_entries (
    message_id TEXT NOT NULL,
    memory_id TEXT NOT NULL,
    PRIMARY KEY (message_id, memory_id),
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE,
    FOREIGN KEY (memory_id) REFERENCES memory_entries(id) ON DELETE CASCADE
);

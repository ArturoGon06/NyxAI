CREATE TABLE IF NOT EXISTS lorebooks (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_lorebooks_updated_at ON lorebooks(updated_at DESC);

CREATE TABLE IF NOT EXISTS lorebook_entries (
    id TEXT PRIMARY KEY NOT NULL,
    lorebook_id TEXT NOT NULL,
    name TEXT NOT NULL,
    keys_json TEXT NOT NULL DEFAULT '[]',
    content TEXT NOT NULL DEFAULT '',
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    priority INTEGER NOT NULL DEFAULT 50,
    case_sensitive INTEGER NOT NULL DEFAULT 0 CHECK (case_sensitive IN (0, 1)),
    match_mode TEXT NOT NULL DEFAULT 'any_key' CHECK (match_mode IN ('any_key', 'all_keys')),
    always_active INTEGER NOT NULL DEFAULT 0 CHECK (always_active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY (lorebook_id) REFERENCES lorebooks(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_lore_entries_book_priority
    ON lorebook_entries(lorebook_id, enabled, priority DESC);

CREATE TABLE IF NOT EXISTS character_lorebooks (
    character_id TEXT NOT NULL,
    lorebook_id TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    PRIMARY KEY (character_id, lorebook_id),
    FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE,
    FOREIGN KEY (lorebook_id) REFERENCES lorebooks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS chat_lorebooks (
    chat_id TEXT NOT NULL,
    lorebook_id TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    PRIMARY KEY (chat_id, lorebook_id),
    FOREIGN KEY (chat_id) REFERENCES chats(id) ON DELETE CASCADE,
    FOREIGN KEY (lorebook_id) REFERENCES lorebooks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS message_lore_entries (
    message_id TEXT NOT NULL,
    lorebook_entry_id TEXT NOT NULL,
    PRIMARY KEY (message_id, lorebook_entry_id),
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE,
    FOREIGN KEY (lorebook_entry_id) REFERENCES lorebook_entries(id) ON DELETE CASCADE
);

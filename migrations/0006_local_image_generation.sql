CREATE TABLE IF NOT EXISTS chat_images (
    id TEXT PRIMARY KEY NOT NULL,
    chat_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    prompt TEXT NOT NULL,
    model TEXT,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY (chat_id) REFERENCES chats(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_chat_images_chat_created_at
    ON chat_images(chat_id, created_at ASC);

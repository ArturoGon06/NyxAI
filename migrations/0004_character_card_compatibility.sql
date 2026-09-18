ALTER TABLE characters ADD COLUMN creator_notes TEXT NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS character_greetings (
    id TEXT PRIMARY KEY NOT NULL,
    character_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE,
    UNIQUE (character_id, position)
);

CREATE INDEX IF NOT EXISTS idx_character_greetings_character_position
    ON character_greetings(character_id, position ASC);

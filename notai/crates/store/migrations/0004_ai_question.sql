-- AI sorusu (Gökhan, 2026-11-07): kayıt net değilse AI kullanıcıya soru sorar. Öneri türlerine
-- 'question' eklenir (CHECK kısıtı: SQLite'ta tablo yeniden kurulur; satırlar korunur).
CREATE TABLE suggestions_v2 (
    id          TEXT PRIMARY KEY NOT NULL,
    entry_id    TEXT NOT NULL REFERENCES entries (id) ON DELETE CASCADE,
    kind        TEXT NOT NULL CHECK (kind IN ('topic', 'task', 'tag', 'person', 'reminder', 'question')),
    -- core::SuggestionKind JSON'u; okunurken yeniden doğrulanır (soruda cevap da buradadır).
    payload     TEXT NOT NULL CHECK (json_valid(payload)),
    confidence  REAL NOT NULL CHECK (confidence >= 0 AND confidence <= 1),
    decision    TEXT NOT NULL DEFAULT 'pending' CHECK (decision IN ('pending', 'accepted', 'dismissed')),
    decided_at  TEXT,
    created_at  TEXT NOT NULL,
    undo        TEXT CHECK (undo IS NULL OR json_valid(undo)),
    auto        INTEGER NOT NULL DEFAULT 0 CHECK (auto IN (0, 1)),
    CHECK ((decision = 'pending') = (decided_at IS NULL))
);
INSERT INTO suggestions_v2 (id, entry_id, kind, payload, confidence, decision, decided_at, created_at, undo, auto)
    SELECT id, entry_id, kind, payload, confidence, decision, decided_at, created_at, undo, auto FROM suggestions;
DROP TABLE suggestions;
ALTER TABLE suggestions_v2 RENAME TO suggestions;
CREATE INDEX suggestions_by_entry ON suggestions (entry_id);

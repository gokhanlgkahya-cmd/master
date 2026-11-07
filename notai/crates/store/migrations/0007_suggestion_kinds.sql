-- Öneri türüne 'expense' (kayıttan harcama/gelir) eklenir. Tür listesi artık kodda doğrulanır
-- (core::SuggestionKind); her yeni türde tablo yeniden kurulmasın diye CHECK kaldırılır.
CREATE TABLE suggestions_v3 (
    id          TEXT PRIMARY KEY NOT NULL,
    entry_id    TEXT NOT NULL REFERENCES entries (id) ON DELETE CASCADE,
    kind        TEXT NOT NULL CHECK (length(kind) > 0),
    payload     TEXT NOT NULL CHECK (json_valid(payload)),
    confidence  REAL NOT NULL CHECK (confidence >= 0 AND confidence <= 1),
    decision    TEXT NOT NULL DEFAULT 'pending' CHECK (decision IN ('pending', 'accepted', 'dismissed')),
    decided_at  TEXT,
    created_at  TEXT NOT NULL,
    undo        TEXT CHECK (undo IS NULL OR json_valid(undo)),
    auto        INTEGER NOT NULL DEFAULT 0 CHECK (auto IN (0, 1)),
    CHECK ((decision = 'pending') = (decided_at IS NULL))
);
INSERT INTO suggestions_v3 (id, entry_id, kind, payload, confidence, decision, decided_at, created_at, undo, auto)
    SELECT id, entry_id, kind, payload, confidence, decision, decided_at, created_at, undo, auto FROM suggestions;
DROP TABLE suggestions;
ALTER TABLE suggestions_v3 RENAME TO suggestions;
CREATE INDEX suggestions_by_entry ON suggestions (entry_id);

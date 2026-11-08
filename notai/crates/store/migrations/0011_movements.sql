-- Konum hareketleri (2026-11-08): yerler (konu başına ayar) ve hareketler (kayda bağlı).
-- İkisi de JSON taşır; değişiklik günlüğüyle senkronlanır.
CREATE TABLE places (
    topic_id   TEXT PRIMARY KEY NOT NULL,
    info       TEXT NOT NULL CHECK (json_valid(info)),
    updated_at TEXT NOT NULL
);

CREATE TABLE movements (
    id         TEXT PRIMARY KEY NOT NULL,
    entry_id   TEXT NOT NULL,
    at         TEXT NOT NULL,
    data       TEXT NOT NULL CHECK (json_valid(data)),
    settled    INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);
CREATE INDEX movements_by_at ON movements (at);
CREATE UNIQUE INDEX movements_by_entry ON movements (entry_id);

CREATE TABLE change_log_v4 (
    seq        INTEGER PRIMARY KEY AUTOINCREMENT,
    entity     TEXT NOT NULL CHECK (entity IN ('topic', 'entry', 'label', 'suggestion', 'task', 'fact', 'attachment', 'transaction', 'merchant_rule', 'place', 'movement')),
    entity_id  TEXT NOT NULL,
    op         TEXT NOT NULL CHECK (op IN ('upsert', 'delete')),
    payload    TEXT CHECK (payload IS NULL OR json_valid(payload)),
    origin     TEXT NOT NULL,
    at         TEXT NOT NULL
);
INSERT INTO change_log_v4 (seq, entity, entity_id, op, payload, origin, at)
    SELECT seq, entity, entity_id, op, payload, origin, at FROM change_log;
DROP TABLE change_log;
ALTER TABLE change_log_v4 RENAME TO change_log;
CREATE INDEX change_log_by_entity ON change_log (entity, entity_id, seq);

-- Harcamalar (Gökhan, 2026-11-07): hesap hareketleri, öğrenilen satıcı kuralları, yüklenen ekstreler.
CREATE TABLE transactions (
    id           TEXT PRIMARY KEY NOT NULL,
    date         TEXT NOT NULL,
    -- Kuruş; eksi = gider, artı = gelir.
    amount       INTEGER NOT NULL CHECK (amount != 0),
    description  TEXT NOT NULL CHECK (length(trim(description)) > 0),
    category     TEXT NOT NULL,
    scope        TEXT NOT NULL CHECK (scope IN ('is', 'ev', 'kisisel')),
    source       TEXT NOT NULL CHECK (source IN ('manual', 'statement')),
    account      TEXT,
    topic_id     TEXT,
    entry_id     TEXT,
    -- Ekstre satırının mükerrer anahtarı: aynı ekstre ikinci kez yüklenirse satır eklenmez.
    dedupe_key   TEXT UNIQUE,
    -- Elle giriş ekstredeki karşılığıyla eşleşti: raporda sayılmaz.
    matched_to   TEXT,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);
CREATE INDEX transactions_by_date ON transactions (date);

-- Kullanıcının düzelttiği satıcı → kategori/kapsam (öğrenme; sonraki ekstrelerde önce bu uygulanır).
CREATE TABLE merchant_rules (
    merchant    TEXT PRIMARY KEY NOT NULL,
    category    TEXT NOT NULL,
    scope       TEXT NOT NULL CHECK (scope IN ('is', 'ev', 'kisisel')),
    updated_at  TEXT NOT NULL
);

-- Yüklenen ekstreler (kurallarla ayrıştırılamazsa AI/ajan işine dönüşür: state = 'pending').
CREATE TABLE statements (
    id          TEXT PRIMARY KEY NOT NULL,
    account     TEXT NOT NULL,
    file_name   TEXT NOT NULL,
    text        TEXT NOT NULL,
    state       TEXT NOT NULL CHECK (state IN ('done', 'pending', 'failed')),
    imported    INTEGER NOT NULL DEFAULT 0,
    skipped     INTEGER NOT NULL DEFAULT 0,
    matched     INTEGER NOT NULL DEFAULT 0,
    error       TEXT,
    created_at  TEXT NOT NULL
);

-- Değişiklik günlüğü harcamaları ve satıcı kurallarını da taşır (CHECK genişletilir).
CREATE TABLE change_log_v3 (
    seq        INTEGER PRIMARY KEY AUTOINCREMENT,
    entity     TEXT NOT NULL CHECK (entity IN ('topic', 'entry', 'label', 'suggestion', 'task', 'fact', 'attachment', 'transaction', 'merchant_rule')),
    entity_id  TEXT NOT NULL,
    op         TEXT NOT NULL CHECK (op IN ('upsert', 'delete')),
    payload    TEXT CHECK (payload IS NULL OR json_valid(payload)),
    origin     TEXT NOT NULL,
    at         TEXT NOT NULL
);
INSERT INTO change_log_v3 (seq, entity, entity_id, op, payload, origin, at)
    SELECT seq, entity, entity_id, op, payload, origin, at FROM change_log;
DROP TABLE change_log;
ALTER TABLE change_log_v3 RENAME TO change_log;
CREATE INDEX change_log_by_entity ON change_log (entity, entity_id, seq);

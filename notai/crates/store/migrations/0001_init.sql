-- NotAI ilk şema. Zamanlar UTC, sabit biçimli RFC3339 metin ("2026-11-06T09:12:00.000000000Z"):
-- sözlük sırası = zaman sırası. Kimlikler UUID v7 metni.

CREATE TABLE topics (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL CHECK (length(trim(name)) > 0),
    -- Türkçe küçük harfe çevrilmiş ad: "GemOlive" ile "gemolive" aynı konudur.
    name_key    TEXT NOT NULL UNIQUE,
    kind        TEXT NOT NULL CHECK (kind IN ('project', 'area', 'resource', 'archive')),
    due_date    TEXT,
    icon        TEXT,
    pinned      INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    CHECK (kind = 'project' OR due_date IS NULL)
);

CREATE TABLE entries (
    id           TEXT PRIMARY KEY NOT NULL,
    text         TEXT NOT NULL CHECK (length(trim(text)) > 0),
    source       TEXT NOT NULL CHECK (source IN ('app', 'voice', 'whats_app', 'share')),
    captured_at  TEXT NOT NULL,
    topic_id     TEXT REFERENCES topics (id) ON DELETE SET NULL,
    status       TEXT NOT NULL CHECK (status IN ('inbox', 'processed', 'archived', 'trashed')),
    trashed_at   TEXT,
    -- WhatsApp yinelemelerini yakalamak için tekil anahtar (isteğe bağlı).
    dedupe_key   TEXT UNIQUE,
    -- AI iş kuyruğu durumu: bekliyor / bitti / başarısız.
    ai_state     TEXT NOT NULL DEFAULT 'pending' CHECK (ai_state IN ('pending', 'done', 'failed')),
    ai_attempts  INTEGER NOT NULL DEFAULT 0 CHECK (ai_attempts >= 0),
    ai_next_at   TEXT,
    ai_error     TEXT,
    updated_at   TEXT NOT NULL,
    CHECK ((status = 'trashed') = (trashed_at IS NOT NULL))
);
CREATE INDEX entries_by_time ON entries (captured_at);
CREATE INDEX entries_by_topic ON entries (topic_id, captured_at);
CREATE INDEX entries_ai_queue ON entries (ai_state, ai_next_at);

-- Tam metin arama (Ara/Sor için; v0.4'te anlamsal arama eklenecek).
CREATE VIRTUAL TABLE entries_fts USING fts5 (
    text,
    content = 'entries',
    content_rowid = 'rowid',
    tokenize = 'unicode61 remove_diacritics 2'
);
CREATE TRIGGER entries_fts_insert AFTER INSERT ON entries BEGIN
    INSERT INTO entries_fts (rowid, text) VALUES (new.rowid, new.text);
END;
CREATE TRIGGER entries_fts_delete AFTER DELETE ON entries BEGIN
    INSERT INTO entries_fts (entries_fts, rowid, text) VALUES ('delete', old.rowid, old.text);
END;
CREATE TRIGGER entries_fts_update AFTER UPDATE OF text ON entries BEGIN
    INSERT INTO entries_fts (entries_fts, rowid, text) VALUES ('delete', old.rowid, old.text);
    INSERT INTO entries_fts (rowid, text) VALUES (new.rowid, new.text);
END;

CREATE TABLE entry_labels (
    entry_id  TEXT NOT NULL REFERENCES entries (id) ON DELETE CASCADE,
    kind      TEXT NOT NULL CHECK (kind IN ('tag', 'person')),
    name      TEXT NOT NULL CHECK (length(trim(name)) > 0),
    PRIMARY KEY (entry_id, kind, name)
);

CREATE TABLE suggestions (
    id          TEXT PRIMARY KEY NOT NULL,
    entry_id    TEXT NOT NULL REFERENCES entries (id) ON DELETE CASCADE,
    kind        TEXT NOT NULL CHECK (kind IN ('topic', 'task', 'tag', 'person', 'reminder')),
    -- core::SuggestionKind JSON'u; okunurken yeniden doğrulanır.
    payload     TEXT NOT NULL CHECK (json_valid(payload)),
    confidence  REAL NOT NULL CHECK (confidence >= 0 AND confidence <= 1),
    decision    TEXT NOT NULL DEFAULT 'pending' CHECK (decision IN ('pending', 'accepted', 'dismissed')),
    decided_at  TEXT,
    created_at  TEXT NOT NULL,
    CHECK ((decision = 'pending') = (decided_at IS NULL))
);
CREATE INDEX suggestions_by_entry ON suggestions (entry_id);

CREATE TABLE tasks (
    id          TEXT PRIMARY KEY NOT NULL,
    entry_id    TEXT REFERENCES entries (id) ON DELETE SET NULL,
    title       TEXT NOT NULL CHECK (length(trim(title)) > 0),
    topic_id    TEXT REFERENCES topics (id) ON DELETE SET NULL,
    due_date    TEXT,
    due_time    TEXT,
    priority    TEXT NOT NULL CHECK (priority IN ('low', 'normal', 'high')),
    recurrence  TEXT CHECK (recurrence IS NULL OR json_valid(recurrence)),
    reminder    TEXT CHECK (reminder IS NULL OR json_valid(reminder)),
    state       TEXT NOT NULL CHECK (state IN ('open', 'done', 'cancelled')),
    closed_at   TEXT,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    CHECK ((state = 'open') = (closed_at IS NULL)),
    CHECK (due_time IS NULL OR due_date IS NOT NULL)
);
CREATE INDEX tasks_open_by_due ON tasks (state, due_date);

CREATE TABLE profile_facts (
    id          TEXT PRIMARY KEY NOT NULL,
    text        TEXT NOT NULL CHECK (length(trim(text)) > 0),
    topic_id    TEXT REFERENCES topics (id) ON DELETE SET NULL,
    status      TEXT NOT NULL CHECK (status IN ('proposed', 'confirmed')),
    created_at  TEXT NOT NULL
);

-- Değişiklik günlüğü: her yazma buraya da düşer (senkron ve denetim için).
CREATE TABLE change_log (
    seq        INTEGER PRIMARY KEY AUTOINCREMENT,
    entity     TEXT NOT NULL CHECK (entity IN ('topic', 'entry', 'label', 'suggestion', 'task', 'fact')),
    entity_id  TEXT NOT NULL,
    op         TEXT NOT NULL CHECK (op IN ('upsert', 'delete')),
    payload    TEXT CHECK (payload IS NULL OR json_valid(payload)),
    origin     TEXT NOT NULL,
    at         TEXT NOT NULL
);
CREATE INDEX change_log_by_entity ON change_log (entity, entity_id, seq);

-- Küçük ayarlar ve durumlar (ör. sabah planının son gönderildiği gün).
CREATE TABLE kv (
    key    TEXT PRIMARY KEY NOT NULL,
    value  TEXT NOT NULL
);

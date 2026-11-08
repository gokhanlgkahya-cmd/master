-- Ses → yazı kuyruğu (2026-11-08). Sesli kaydın ses eki geldiğinde (yerelde ya da senkronla)
-- açılır; Whisper'ı olan cihaz (ev sunucusu) işler, metni düzeltir. Senkronlanmaz.
CREATE TABLE transcripts (
    entry_id      TEXT PRIMARY KEY NOT NULL,
    attachment_id TEXT NOT NULL,
    state         TEXT NOT NULL DEFAULT 'pending' CHECK (state IN ('pending', 'done', 'failed')),
    attempts      INTEGER NOT NULL DEFAULT 0,
    next_at       TEXT,
    error         TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);
CREATE INDEX transcripts_queue ON transcripts (state, next_at);

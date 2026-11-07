-- v0.2–v0.4: hatırlatma bildirimi, ekler, anlamsal arama vektörleri, otonom kararların geri alınması.

-- Hatırlatma bildiriminin gönderildiği an (aynı hatırlatma iki kez çalmasın).
ALTER TABLE tasks ADD COLUMN reminder_fired_at TEXT;

-- Kayda iliştirilen dosyalar (fotoğraf, belge) ve konum/kişi gibi yapılandırılmış ekler.
CREATE TABLE attachments (
    id          TEXT PRIMARY KEY NOT NULL,
    entry_id    TEXT NOT NULL REFERENCES entries (id) ON DELETE CASCADE,
    kind        TEXT NOT NULL CHECK (kind IN ('photo', 'document', 'location', 'contact', 'audio')),
    name        TEXT NOT NULL CHECK (length(trim(name)) > 0),
    mime        TEXT NOT NULL,
    -- Dosya içeriği (en fazla 10 MB; büyük dosyalar v1'de nesne deposuna taşınır).
    data        BLOB,
    -- Konum ("41.0082,28.9784") ya da kişi adı gibi metin değer.
    value       TEXT,
    created_at  TEXT NOT NULL,
    CHECK (data IS NOT NULL OR value IS NOT NULL),
    CHECK (data IS NULL OR length(data) <= 10485760)
);
CREATE INDEX attachments_by_entry ON attachments (entry_id);

-- Anlamsal arama için kayıt vektörleri (little-endian f32 dizisi).
CREATE TABLE entry_embeddings (
    entry_id  TEXT PRIMARY KEY NOT NULL REFERENCES entries (id) ON DELETE CASCADE,
    model     TEXT NOT NULL,
    dims      INTEGER NOT NULL CHECK (dims > 0),
    vector    BLOB NOT NULL CHECK (length(vector) = dims * 4),
    created_at TEXT NOT NULL
);

-- Kabul edilen önerinin geri alınması için gereken önceki durum (ör. kaydın eski konusu).
ALTER TABLE suggestions ADD COLUMN undo TEXT CHECK (undo IS NULL OR json_valid(undo));
-- Otonom modda uygulandı mı (kullanıcı değil sistem kabul etti).
ALTER TABLE suggestions ADD COLUMN auto INTEGER NOT NULL DEFAULT 0 CHECK (auto IN (0, 1));

-- Değişiklik günlüğü artık ekleri de taşır: CHECK kısıtı genişletilir (SQLite'ta tablo yeniden kurulur;
-- sıra numaraları korunur).
CREATE TABLE change_log_v2 (
    seq        INTEGER PRIMARY KEY AUTOINCREMENT,
    entity     TEXT NOT NULL CHECK (entity IN ('topic', 'entry', 'label', 'suggestion', 'task', 'fact', 'attachment')),
    entity_id  TEXT NOT NULL,
    op         TEXT NOT NULL CHECK (op IN ('upsert', 'delete')),
    payload    TEXT CHECK (payload IS NULL OR json_valid(payload)),
    origin     TEXT NOT NULL,
    at         TEXT NOT NULL
);
INSERT INTO change_log_v2 (seq, entity, entity_id, op, payload, origin, at)
    SELECT seq, entity, entity_id, op, payload, origin, at FROM change_log;
DROP TABLE change_log;
ALTER TABLE change_log_v2 RENAME TO change_log;
CREATE INDEX change_log_by_entity ON change_log (entity, entity_id, seq);

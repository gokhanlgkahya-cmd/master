-- Varlıklar (Gökhan, 2026-11-07): varlık bilgisi konunun içinde (senkronlanır); değer geçmişi
-- (günlük, 30 günlük değişim için) her cihazda konu değeri değiştikçe yerelde tutulur.
ALTER TABLE topics ADD COLUMN asset TEXT CHECK (asset IS NULL OR json_valid(asset));
CREATE TABLE asset_values (
    topic_id  TEXT NOT NULL,
    as_of     TEXT NOT NULL,
    value     INTEGER NOT NULL,
    PRIMARY KEY (topic_id, as_of)
);

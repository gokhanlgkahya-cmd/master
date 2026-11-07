-- Fiyat / ürün takibi (Gökhan, 2026-11-07): takip ayarı konunun içinde (konu anlık görüntüsüyle
-- senkronlanır); son kontrol durumu yalnız takibi çalıştıran yerde (senkronlanmaz).
ALTER TABLE topics ADD COLUMN watch TEXT CHECK (watch IS NULL OR json_valid(watch));
CREATE TABLE watch_state (
    topic_id  TEXT PRIMARY KEY NOT NULL,
    state     TEXT NOT NULL CHECK (json_valid(state))
);

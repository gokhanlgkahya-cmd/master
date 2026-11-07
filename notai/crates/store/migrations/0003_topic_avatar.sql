-- Revizyon 3 (Gökhan, 2026-11-07): konu profil fotoğrafı (WhatsApp profili gibi).
-- Küçültülmüş görsel (≈96 px JPEG) data: URL olarak; konu anlık görüntüsüyle senkronlanır.
ALTER TABLE topics ADD COLUMN avatar TEXT;

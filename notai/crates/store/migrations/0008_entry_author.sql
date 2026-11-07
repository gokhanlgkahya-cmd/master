-- NotAI'nin yazdığı kayıtlar (Gökhan, 2026-11-07): ör. fiyat takibinin konuya getirdiği mesajlar.
-- NULL = kullanıcı. Kayıtlar tablosu yeniden kurulmaz (veri güvenliği); yalnız sütun eklenir.
ALTER TABLE entries ADD COLUMN author TEXT CHECK (author IS NULL OR author = 'notai');

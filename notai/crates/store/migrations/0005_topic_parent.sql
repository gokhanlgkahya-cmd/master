-- Konu hiyerarşisi (Gökhan, 2026-11-07): "Sabun işi" → "Malzeme hareketleri" → … Kayıt tek
-- konuya bağlıdır; üst konuların akışında da görünür. Senkronda alt konu üstünden önce gelebilir,
-- bu yüzden yabancı anahtar yok (bütünlüğü depo denetler). Ad benzersizliği artık üst konu
-- içindedir: alt konuların name_key'i "<üst id>/<küçük harf ad>" (en üsttekiler değişmez).
ALTER TABLE topics ADD COLUMN parent_id TEXT;
CREATE INDEX topics_by_parent ON topics (parent_id);

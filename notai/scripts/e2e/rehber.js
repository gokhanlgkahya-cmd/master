const { chromium } = require('playwright');
const D = require('path').join(__dirname, 'out', '/rehber'); require('fs').mkdirSync(D, { recursive: true });
const URL = 'http://127.0.0.1:18080/?api=http://127.0.0.1:18787&token=gizli';
const PDF = require('path').resolve(__dirname,'../../crates/server/tests/fixtures/ornek-ekstre.pdf');
const done = [], fails = [];
let p, ctx;
const idle = (ms = 700) => p.waitForTimeout(ms);
const tab = n => p.locator('nav.tabbar button', { hasText: n }).click();
async function shot(name, fn, full = false) {
  try { await fn(); await idle(); await p.screenshot({ path: `${D}/${name}.png`, fullPage: full }); done.push(name); }
  catch (e) { fails.push(name + ' :: ' + String(e.message).split('\n')[0]); try { await p.screenshot({ path: `${D}/HATA-${name}.png` }); } catch {} }
}
const closeSheet = async () => { if (await p.locator('.scrim').count()) await p.locator('.scrim').click({ position: { x: 20, y: 20 } }); await idle(); };
const home = async () => {
  for (let i = 0; i < 4 && !(await p.locator('.searchbar').count()); i++) {
    if (await p.getByLabel('Kapat', { exact: true }).count()) { await p.getByLabel('Kapat', { exact: true }).click(); }
    else if (await p.locator('nav.tabbar').count()) { await tab('Günlük'); }
    else { await p.getByLabel('Geri').first().click(); }
    await idle(400);
  }
  await p.locator('.chat').first().waitFor();
};
(async () => {
  const b = await chromium.launch({ args: ['--use-fake-ui-for-media-stream', '--use-fake-device-for-media-stream', '--lang=tr-TR'], env: { ...process.env, LANG: 'tr_TR.UTF-8', LANGUAGE: 'tr' } });
  ctx = await b.newContext({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 2, locale: 'tr-TR', timezoneId: 'Europe/Istanbul',
    geolocation: { latitude: 40.99, longitude: 29.03, accuracy: 15 } });
  await ctx.grantPermissions(['microphone', 'geolocation'], { origin: 'http://127.0.0.1:18080' });
  p = await ctx.newPage();
  await p.goto(URL); await p.locator('.chat').first().waitFor(); await p.evaluate(() => document.fonts.ready);

  await shot('01-ana-ekran', async () => {});
  await shot('02-yeni-kayit-yaz', async () => {
    await p.getByLabel('Yeni kayıt').click(); await idle(500);
    await p.keyboard.type('Cuma 9da kombi faturasını öde');
  });
  await shot('03-kayit-gitti', async () => { await p.keyboard.press('Enter'); await p.locator('.bubble', { hasText: 'kombi faturasını' }).waitFor(); });
  await shot('04-ai-onerileri', async () => {
    await p.locator('.bubble', { hasText: 'kostik siparişi' }).first().scrollIntoViewIfNeeded();
    await p.locator('.bubble', { hasText: 'kostik siparişi' }).first().evaluate(e => e.scrollIntoView({ block: 'start' }));
  });
  await shot('05-ai-sorusu', async () => {
    await p.locator('.aimsg').first().evaluate(e => e.scrollIntoView({ block: 'center' }));
    await p.locator('.aimsg__input').first().fill('Ambalajcı Ahmet, perşembe 14:00');
  });
  await p.getByLabel('Cevabı gönder').first().click().catch(() => {}); await idle(800);
  await shot('06-not-detayi', async () => { await p.locator('.bubble', { hasText: 'kostik siparişi' }).first().click(); await p.locator('.sheet').waitFor(); });
  await closeSheet();
  await shot('07-ekleme-paneli', async () => { await p.getByLabel('Ekle', { exact: true }).click(); await p.locator('.attach').waitFor(); });
  await p.getByLabel('Ekleme panelini kapat').click().catch(() => {}); await idle();
  await shot('08-sesli-not', async () => { await p.getByLabel('Sesli yakala').click(); await p.locator('.voicestrip').waitFor(); await idle(2500); });
  await p.locator('.voicestrip button', { hasText: 'İptal' }).click().catch(() => {}); await idle();
  await home();
  await shot('09-ust-konu', async () => { await p.locator('.chat', { hasText: 'Sabun işi' }).first().click(); await p.locator('.chat', { hasText: 'Malzeme hareketleri' }).waitFor(); });
  await shot('10-alt-konu-yazismasi', async () => { await p.locator('.chat', { hasText: 'Malzeme hareketleri' }).click(); await p.locator('.bubble').first().waitFor(); });
  await shot('11-konu-menusu', async () => { await p.getByLabel('Konu menüsü').click(); await p.locator('.sheet').waitFor(); }, false);
  await closeSheet();
  await home();
  await shot('12-yeni-konu', async () => { await p.getByLabel('Yeni konu').click(); await p.locator('.sheet').waitFor(); await p.locator('.sheet input[type=text]').first().fill('Bahçe düzenlemesi'); });
  await closeSheet();
  await shot('13-gorevler', async () => { await tab('Görevler'); await p.locator('.task').first().waitFor(); });
  await shot('14-gorev-duzenle', async () => { await p.locator('.task', { hasText: 'Kombi servisini ara' }).locator('.task__txt, .task__title').first().click(); await p.locator('.sheet').waitFor(); });
  await closeSheet();
  await shot('15-takvim-hafta', async () => { await tab('Takvim'); await idle(); });
  await shot('16-takvim-ay', async () => { await p.locator('.switch2 button', { hasText: 'Ay' }).click(); await idle(); });
  await shot('17-hatirlatmalar', async () => { await tab('Hatırlatma'); await idle(); });
  await shot('18-ara-sor', async () => {
    await tab('Ara/Sor'); await idle();
    const inp = p.locator('input[type=search], .askbar input, input').first();
    await inp.fill('kostik'); await inp.press('Enter'); await idle(1500);
  });
  await home();
  await shot('19-haftalik', async () => { await p.locator('.chat', { hasText: 'Haftalık gözden geçirme' }).click(); await idle(); });
  await home();
  await shot('20-harcamalar', async () => {
    await p.locator('.chat', { hasText: 'Harcamalar' }).first().click(); await idle();
    await p.locator('.fin-actions input[type=file]').setInputFiles(PDF); await idle(2000);
    await p.getByLabel('Önceki ay').click(); await idle(1200);
  });
  await shot('21-varliklar', async () => { await p.locator('.seg3 button', { hasText: 'Varlıklar' }).click(); await idle(); });
  await home();
  await p.getByLabel('Yeni kayıt').click(); await idle(400);
  await p.keyboard.type('Evden çıkıyorum metroyla'); await p.keyboard.press('Enter'); await idle(2500);
  await ctx.setGeolocation({ latitude: 41.043, longitude: 29.009, accuracy: 20 });
  await p.keyboard.type('işe geldim'); await p.keyboard.press('Enter'); await idle(2500);
  await shot('22-hareket-kaydi', async () => { await p.locator('.bubble', { hasText: 'işe geldim' }).waitFor(); });
  await home();
  await shot('23-hareketler', async () => {
    await p.locator('.chat', { hasText: 'Harcamalar' }).first().click(); await idle();
    await p.locator('.seg3 button', { hasText: 'Hareketler' }).click(); await idle(1000);
  });
  await home();
  await shot('24-fiyat-takibi', async () => { await p.locator('.chat', { hasText: 'Pixel 10 Pro' }).first().click(); await p.locator('.bubble--notai').first().waitFor(); });
  await home();
  await shot('25-ayarlar', async () => { await p.getByLabel('Ayarlar').click(); await idle(); });
  await home();
  await p.emulateMedia({ colorScheme: 'dark' });
  await shot('26-koyu-tema', async () => { await idle(); });
  console.log('TAMAM:', done.length, done.join(' '));
  if (fails.length) console.log('HATALAR:\n' + fails.join('\n'));
  await b.close();
})();

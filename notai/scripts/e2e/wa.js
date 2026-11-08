const { chromium } = require('playwright');
const D = require('path').join(__dirname, 'out'); require('fs').mkdirSync(D, { recursive: true });
const URL = 'http://127.0.0.1:18080/?api=http://127.0.0.1:18787&token=gizli';
const results = [], logs = [];
let p;
async function step(name, fn) {
  try { await fn(); results.push('OK   ' + name); }
  catch (e) { results.push('FAIL ' + name + ' :: ' + String(e.message).split('\n')[0]); try { await p.screenshot({ path: `${D}/fail-${name.replace(/\W+/g,'_')}.png` }); } catch {} }
}
const idle = () => p.waitForTimeout(700);
const tab = n => p.locator('nav.tabbar button', { hasText: n }).click();
async function noError() { if (await p.locator('.error-note').count()) throw new Error(await p.locator('.error-note').first().innerText()); }
(async () => {
  const b = await chromium.launch();
  const ctx = await b.newContext({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 2 });
  p = await ctx.newPage();
  p.on('console', m => { if (m.type() === 'error') logs.push(m.text()); });
  p.on('pageerror', e => logs.push('pageerror: ' + e.message));
  await p.goto(URL); await p.locator('.chat').first().waitFor();

  await step('liste: Ana Günlük + Haftalık iğneli, yuvarlak profiller, ayraç/tür etiketi yok', async () => {
    const names = await p.locator('.chat__name').allInnerTexts();
    if (names[0] !== 'Ana Günlük' || names[1] !== 'Haftalık gözden geçirme') throw new Error(names.join(','));
    if (await p.locator('.chat__pin').count() < 2) throw new Error('iğne yok');
    const r = await p.locator('.chat .avatar').first().evaluate(e => getComputedStyle(e).borderRadius);
    if (!r.includes('9999') && !r.includes('50%')) throw new Error('yuvarlak değil: ' + r);
    if (await p.locator('.topic__kind, .grouphead, .card--top').count()) throw new Error('eski yapı kalmış');
    if (await p.locator('.composer').count()) throw new Error('ana ekranda yazma çubuğu var');
  });

  await step('rozet: karar bekleyen kayıt sayısı', async () => {
    const row = p.locator('.chat', { hasText: 'Kostik tedarik' });
    const t = await row.locator('.badge').innerText();
    if (t !== '1') throw new Error('rozet: ' + t);
  });

  await step('hap filtre: İşlenmemiş → yalnız bekleyenler', async () => {
    await p.locator('.pill', { hasText: 'İşlenmemiş' }).click(); await idle();
    const names = await p.locator('.chat__name').allInnerTexts();
    if (!names.includes('Kostik tedarik') || names.includes('GemOlive') || names.includes('Haftalık gözden geçirme')) throw new Error(names.join(','));
    await p.screenshot({ path: `${D}/wa-02-islenmemis.png` });
    await p.locator('.pill', { hasText: 'Proje' }).click(); await idle();
    const proj = await p.locator('.chat__name').allInnerTexts();
    if (proj.includes('Ana Günlük') || proj.includes('GemOlive')) throw new Error('proje: ' + proj.join(','));
    await p.locator('.pill', { hasText: 'Tümü' }).click(); await idle();
  });

  await step('arama çubuğu → Ara/Sor', async () => {
    await p.locator('.searchbar').click(); await idle();
    await p.locator('input[type=search], .search input').first().waitFor({ timeout: 3000 });
    await tab('Günlük'); await p.locator('.chat').first().waitFor();
  });

  await step('yeni kayıt düğmesi → Ana Günlük, klavye açık, kayıt düşer', async () => {
    await p.getByLabel('Yeni kayıt').click();
    await p.locator('.flowbar__name', { hasText: 'Ana Günlük' }).waitFor();
    await p.waitForTimeout(400);
    const focused = await p.evaluate(() => document.activeElement && document.activeElement.classList.contains('field__input'));
    if (!focused) throw new Error('alan odakta değil');
    await p.screenshot({ path: `${D}/wa-04-yazma.png` });
    await p.keyboard.type('Matbaaya etiket provası için yarın uğra');
    await p.screenshot({ path: `${D}/wa-04b-yazarken.png` });
    await p.keyboard.press('Enter'); await idle(); await noError();
    await p.locator('.bubble', { hasText: 'Matbaaya etiket provası' }).waitFor();
  });

  await step('yazma çubuğu: ataç + kamera alanın içinde, mikrofon dışarıda', async () => {
    const inField = await p.locator('.field--pill [aria-label="Ekle"], .field--pill [aria-label="Kamera"]').count();
    const mic = await p.locator('.composer > .mic').count();
    if (inField !== 2 || mic !== 1) throw new Error(`alan içi=${inField} mic=${mic}`);
    await p.getByLabel('Ekle', { exact: true }).click();
    await p.locator('.tile', { hasText: 'Etkinlik' }).waitFor();
    await p.getByLabel('Ekleme panelini kapat').click();
    await p.getByLabel('Geri').click(); await p.locator('.chat').first().waitFor();
  });

  await step('konu menüsü: profil fotoğrafı yükle', async () => {
    await p.locator('.chat', { hasText: 'GemOlive' }).click();
    await p.getByLabel('Konu menüsü').click();
    await p.locator('.sheet .setrow', { hasText: 'Profil fotoğrafı' }).waitFor();
    await p.screenshot({ path: `${D}/wa-05-konu-menu.png` });
    await p.locator('.sheet input[type=file]').setInputFiles(D + '/logo.png');
    await p.waitForTimeout(1500); await noError();
    const src = await p.locator('.flowbar .avatar img').getAttribute('src');
    if (!src || !src.startsWith('data:image/jpeg')) throw new Error('başlıkta foto yok');
    results.push('     foto boyutu: ' + src.length + ' karakter');
    await p.getByLabel('Geri').click(); await p.locator('.chat').first().waitFor();
    if (!await p.locator('.chat', { hasText: 'GemOlive' }).locator('.avatar img').count()) throw new Error('listede foto yok');
  });

  await step('konu menüsü: simgeyi değiştir → Kalp, sonra Otomatik', async () => {
    await p.locator('.chat', { hasText: 'Araba bakımı' }).click();
    await p.getByLabel('Konu menüsü').click();
    await p.locator('.sheet .setrow', { hasText: 'Simgeyi değiştir' }).click();
    await p.locator('.icongrid').waitFor();
    await p.screenshot({ path: `${D}/wa-06-simge-sec.png` });
    await p.getByLabel('Simge: İlişki').click(); await idle(); await noError();
    const use = await p.locator('.flowbar .avatar use').getAttribute('href');
    if (use !== '#i-heart') throw new Error('simge: ' + use);
    await p.getByLabel('Konu menüsü').click();
    await p.locator('.sheet .setrow', { hasText: 'Simgeyi değiştir' }).click();
    await p.getByLabel('Otomatik simge').click(); await idle();
    const back = await p.locator('.flowbar .avatar use').getAttribute('href');
    if (back !== '#i-car') throw new Error('otomatiğe dönmedi: ' + back);
    await p.getByLabel('Geri').click(); await p.locator('.chat').first().waitFor();
  });

  await step('son ekran görüntüleri', async () => {
  await p.screenshot({ path: `${D}/wa-01-ana.png` });
  await p.emulateMedia({ colorScheme: 'dark' }); await idle();
  await p.screenshot({ path: `${D}/wa-01-ana-koyu.png` });
  await p.locator('.chat', { hasText: 'Kostik tedarik' }).click(); await p.locator('.bubble').first().waitFor(); await idle();
  await p.screenshot({ path: `${D}/wa-03-akis-koyu.png` });
  await p.emulateMedia({ colorScheme: 'light' }); await idle();
  await p.screenshot({ path: `${D}/wa-03-akis.png` });
  });
  console.log(results.join('\n'));
  console.log('--- konsol ---\n' + logs.join('\n'));
  await b.close();
})();

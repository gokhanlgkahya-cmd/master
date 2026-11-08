const { chromium } = require('playwright');
const D = require('path').join(__dirname, 'out'); require('fs').mkdirSync(D, { recursive: true });
const URL = 'http://127.0.0.1:18080/?api=http://127.0.0.1:18787&token=gizli';
const ORIGIN = 'http://127.0.0.1:18080';
const results = []; let p, ctx;
async function step(name, fn) {
  try { await fn(); results.push('OK   ' + name); }
  catch (e) { results.push('FAIL ' + name + ' :: ' + String(e.message).split('\n')[0]); try { await p.screenshot({ path: `${D}/fail-${name.replace(/\W+/g,'_')}.png` }); } catch {} }
}
const idle = (ms = 800) => p.waitForTimeout(ms);
const HOME = { latitude: 40.990, longitude: 29.030, accuracy: 15 };
const WORK = { latitude: 41.043, longitude: 29.009, accuracy: 20 };
async function write(text) {
  await p.getByLabel('Yeni kayıt').click(); await idle(400);
  await p.keyboard.type(text); await p.keyboard.press('Enter');
  await p.locator('.bubble', { hasText: text }).first().waitFor();
}
async function home() {
  for (let i = 0; i < 3 && !(await p.locator('.searchbar').count()); i++) { await p.getByLabel('Geri').click(); await idle(400); }
}
(async () => {
  const b = await chromium.launch();
  ctx = await b.newContext({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 2, geolocation: HOME });
  await ctx.grantPermissions(['geolocation'], { origin: ORIGIN });
  p = await ctx.newPage();
  const errs = []; p.on('pageerror', e => errs.push(e.message));
  await p.goto(URL); await p.locator('.chat').first().waitFor();
  await step('"Evden çıkıyorum" (konum açık) → Ev öğrenildi + NotAI sorusu', async () => {
    await write('Evden çıkıyorum');
    await p.locator('.info-note', { hasText: 'Ev konumu öğrenildi' }).waitFor({ timeout: 12000 });
    await p.locator('.aimsg', { hasText: 'Nereye, neyle gidiyorsun' }).waitFor({ timeout: 6000 });
    await p.screenshot({ path: `${D}/mv-01-soru.png` });
  });
  await step('soruyu cevapla: "metroyla işe"', async () => {
    await p.locator('.aimsg__input').fill('metroyla işe');
    await p.getByLabel('Cevabı gönder').click(); await idle(1200);
    await p.locator('.aimsg__answer', { hasText: 'metroyla işe' }).waitFor();
  });
  await step('"işe geldim" (konum kapalı) → Konumu aç uyarısı + şerit', async () => {
    const main = p;
    const ctx2 = await b.newContext({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 2 });
    p = await ctx2.newPage();
    await p.goto(URL); await p.locator('.chat').first().waitFor();
    await write('işe geldim');
    await p.locator('.info-note', { hasText: 'Konum kapalı' }).waitFor({ timeout: 12000 });
    await p.screenshot({ path: `${D}/mv-02-konum-kapali.png` });
    await home();
    await p.reload(); await p.locator('.chat').first().waitFor();
    const t = await p.locator('.notice__title').innerText();
    if (!t.includes('Konumu aç · İş')) throw new Error(t);
    await p.screenshot({ path: `${D}/mv-03-serit.png` });
    await ctx2.close(); p = main;
  });
  await step('konum açık, işte: "işteyim" → İş öğrenildi; akşam evde teyit', async () => {
    await ctx.setGeolocation(WORK);
    await home();
    await write('Ofisteyim');
    await p.locator('.info-note', { hasText: 'İş konumu öğrenildi' }).waitFor({ timeout: 12000 });
    await home();
    await write('işten çıktım arabayla');
    await idle(1500);
    await ctx.setGeolocation({ latitude: 40.9903, longitude: 29.0301, accuracy: 25 });
    await home();
    await write('eve geldim');
    await p.locator('.info-note', { hasText: 'teyit edildi: Ev' }).waitFor({ timeout: 12000 });
  });
  await step('Harcamalar → Hareketler raporu', async () => {
    await home();
    await p.locator('.chat', { hasText: 'Harcamalar' }).first().click();
    await p.locator('.seg3 button', { hasText: 'Hareketler' }).click();
    await p.locator('.grouphead', { hasText: 'Yerler' }).waitFor();
    const trips = await p.locator('.fin-tile__val').first().innerText();
    if (Number(trips) < 2) throw new Error('yolculuk ' + trips);
    await p.screenshot({ path: `${D}/mv-04-rapor.png`, fullPage: true });
  });
  await step('ana ekranda Hareketler konusu (Ev, İş alt konular)', async () => {
    await home();
    await p.locator('.chat', { hasText: 'Hareketler' }).first().click();
    await p.locator('.chat', { hasText: 'İş' }).first().waitFor();
    await p.screenshot({ path: `${D}/mv-05-hub.png` });
  });
  console.log(results.join('\n'));
  if (errs.length) console.log('PAGE ERRORS:', errs);
  await b.close();
})();

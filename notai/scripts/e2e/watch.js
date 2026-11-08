const { chromium } = require('playwright');
const { execFileSync } = require('child_process');
const D = require('path').join(__dirname, 'out'); require('fs').mkdirSync(D, { recursive: true });
const URL = 'http://127.0.0.1:18080/?api=http://127.0.0.1:18787&token=gizli';
const env = { ...process.env, NOTAI_URL: 'http://127.0.0.1:18787', NOTAI_TOKEN: 'gizli' };
const cli = (args, input) => execFileSync(require('path').resolve(__dirname,'../../target/debug/notai'), args, { env, input, encoding: 'utf8' });
const results = []; let p;
async function step(name, fn) {
  try { await fn(); results.push('OK   ' + name); }
  catch (e) { results.push('FAIL ' + name + ' :: ' + String(e.message).split('\n')[0]); try { await p.screenshot({ path: `${D}/fail-${name.replace(/\W+/g,'_')}.png` }); } catch {} }
}
const idle = (ms = 800) => p.waitForTimeout(ms);
(async () => {
  const b = await chromium.launch();
  p = await (await b.newContext({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 2 })).newPage();
  const errs = []; p.on('pageerror', e => errs.push(e.message));
  await p.goto(URL); await p.locator('.chat').first().waitFor();
  let tid;
  await step('konu: Pixel 10 Pro → menü → Fiyat takibi formu', async () => {
    await p.getByLabel('Yeni konu').click();
    await p.locator('.sheet input[type=text]').fill('Pixel 10 Pro');
    await p.locator('.sheet .seg3 button', { hasText: 'Kaynak' }).click();
    await p.locator('.sheet button[type=submit]').click(); await idle();
    await p.locator('.chat', { hasText: 'Pixel 10 Pro' }).click();
    await p.getByLabel('Konu menüsü').click();
    await p.locator('.sheet .setrow', { hasText: 'Fiyat takibi' }).click(); await idle(400);
    await p.locator('.sheet input[type=text]').first().fill('Pixel 10 Pro 256 GB');
    await p.locator('.sheet textarea').fill('https://example.com/pixel-10-pro');
    await p.getByLabel('Hedef fiyat').fill('50.000');
    await p.screenshot({ path: `${D}/watch-01-form.png` });
    await p.locator('.sheet button[type=submit]').click(); await idle();
    await p.locator('.flowbar__sub', { hasText: 'fiyat takibinde' }).waitFor();
  });
  await step('ajan: notai watches → watch-report (ilk sonuç)', async () => {
    const jobs = JSON.parse(cli(['watches']));
    const j = jobs.find(x => x.query === 'Pixel 10 Pro 256 GB'); tid = j.topic_id;
    const out = cli(['watch-report', tid, '-'], JSON.stringify({ offers: [
      { title: 'Google Pixel 10 Pro 256 GB', price: '54.999,00', url: 'https://www.trendyol.com/google/pixel-10-pro', store: 'Trendyol' },
      { title: 'Pixel 10 Pro 256 GB Obsidian', price: '55.499,00', url: 'https://www.hepsiburada.com/pixel-10-pro', store: 'Hepsiburada' } ] }));
    results.push('     ' + out.split('\n')[0]);
  });
  await step('ana ekranda "Takip · Pixel 10 Pro" şeridi; konuda NotAI mesajı + bağlantılar', async () => {
    await p.getByLabel('Geri').click(); await p.locator('.chat').first().waitFor(); await p.reload(); await p.locator('.chat').first().waitFor();
    const title = await p.locator('.notice__title').innerText();
    if (!title.includes('Takip · Pixel 10 Pro')) throw new Error(title);
    await p.locator('.notice__open').click();
    await p.locator('.bubble--notai').first().waitFor();
    const links = await p.locator('.bubble--notai a').count();
    if (links < 2) throw new Error('bağlantı sayısı ' + links);
    await p.screenshot({ path: `${D}/watch-02-mesaj.png` });
  });
  await step('fiyat düşünce ikinci mesaj', async () => {
    // Takip durumunu "dün bakıldı" yapmak için doğrudan rapor (sunucu aralığı zorlamaz).
    cli(['watch-report', tid, '-'], JSON.stringify({ offers: [
      { title: 'Google Pixel 10 Pro 256 GB', price: '51.999,00', url: 'https://www.trendyol.com/google/pixel-10-pro', store: 'Trendyol' } ] }));
    await p.reload(); await p.locator('.chat', { hasText: 'Pixel 10 Pro' }).first().click();
    await p.locator('.bubble--notai', { hasText: 'Fiyat düştü' }).waitFor();
    await p.screenshot({ path: `${D}/watch-03-dusus.png` });
  });
  console.log(results.join('\n')); console.log('pageerror: ' + errs.join(' | '));
  await b.close();
})();

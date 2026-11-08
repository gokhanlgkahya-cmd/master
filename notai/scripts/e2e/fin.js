const { chromium } = require('playwright');
const { execFileSync } = require('child_process');
const D = require('path').join(__dirname, 'out'); require('fs').mkdirSync(D, { recursive: true });
const URL = 'http://127.0.0.1:18080/?api=http://127.0.0.1:18787&token=gizli';
const env = { ...process.env, NOTAI_URL: 'http://127.0.0.1:18787', NOTAI_TOKEN: 'gizli' };
const cli = (args, input) => execFileSync(require('path').resolve(__dirname,'../../target/debug/notai'), args, { env, input, encoding: 'utf8' });
const PDF = require('path').resolve(__dirname,'../../crates/server/tests/fixtures/ornek-ekstre.pdf');
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

  await step('günlükten harcama: "Ayhan Kimya\'ya 1.850 ₺ ödendi" → AI çipi → kabul', async () => {
    await p.getByLabel('Yeni kayıt').click(); await idle(300);
    await p.keyboard.type("3 Ekim'de Ayhan Kimya'ya 1.850 ₺ ödendi"); await p.keyboard.press('Enter'); await idle();
    const job = JSON.parse(cli(['jobs', '--limit', '10'])).find(j => j.text.includes('1.850 ₺ ödendi'));
    cli(['answer', job.entry_id, '-'], JSON.stringify({ topic: null, tasks: [], tags: [], people: [{ name: 'Ayhan Kimya', confidence: 0.8 }],
      reminder: null, question: null, expense: { amount: 1850, direction: 'gider', category: 'tedarik', scope: 'is', date: '2026-10-03', confidence: 0.9 } }));
    await p.reload(); await p.locator('.chat', { hasText: 'Ana Günlük' }).click();
    const chip = p.locator('.chip', { hasText: 'Harcama · 1.850,00' });
    await chip.waitFor();
    await p.screenshot({ path: `${D}/fin-01-cip.png` });
    await chip.locator('.chip__body').click(); await idle();
    await p.getByLabel('Geri').click(); await p.locator('.chat').first().waitFor();
  });
  await step('Harcamalar → Ekim 2026 → PDF ekstre yükle', async () => {
    await p.locator('.chat', { hasText: 'Harcamalar' }).click(); await idle();
    // Bu ay (Kasım) boş; Ekim'e git.
    await p.getByLabel('Önceki ay').click(); await idle();
    await p.locator('.flowbar__sub', { hasText: 'Ekim 2026' }).waitFor();
    await p.locator('.fin-actions input[type=file]').setInputFiles(PDF); await idle(1800);
    const msg = await p.locator('.fin-msg').innerText();
    if (!msg.includes('10 hareket eklendi') || !msg.includes('1 elle girilen ödemeyle eşleşti')) throw new Error(msg);
    results.push('     ' + msg);
    await p.locator('.fin-tile').first().waitFor();
    await p.screenshot({ path: `${D}/fin-02-rapor.png`, fullPage: false });
  });
  await step('aynı PDF tekrar → mükerrer eklenmedi', async () => {
    await p.locator('.fin-actions input[type=file]').setInputFiles(PDF); await idle(1500);
    const msg = await p.locator('.fin-msg').innerText();
    if (!msg.includes('0 hareket eklendi, 10 zaten vardı')) throw new Error(msg);
    const n = await p.locator('.fin-tx').count();
    if (n !== 11) throw new Error('hareket sayısı ' + n);
  });
  await step('düzeltme: Trendyol → İş (öğrenilir)', async () => {
    const row = p.locator('.fin-tx', { hasText: 'TRENDYOL' });
    await row.getByLabel(/^Kapsam:/).selectOption('is'); await idle(1200);
    const isTotal = await p.locator('.fin-row', { hasText: 'İş' }).first().locator('.fin-row__amt').innerText();
    results.push('     İş toplamı: ' + isTotal);
  });
  await step('ekran görüntüleri (yorumlar, dağılım, hareketler)', async () => {
    await p.locator('.body.scroll').evaluate(e => e.scrollTop = 0);
    await p.screenshot({ path: `${D}/fin-02-rapor.png` });
    await p.locator('.grouphead', { hasText: 'Hareketler' }).scrollIntoViewIfNeeded(); await idle(300);
    await p.screenshot({ path: `${D}/fin-03-hareketler.png` });
  });
  console.log(results.join('\n')); console.log('pageerror: ' + errs.join(' | '));
  await b.close();
})();

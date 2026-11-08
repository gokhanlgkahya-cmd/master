const { chromium } = require('playwright');
const { execFileSync } = require('child_process');
const D = require('path').join(__dirname, 'out'); require('fs').mkdirSync(D, { recursive: true });
const URL = 'http://127.0.0.1:18080/?api=http://127.0.0.1:18787&token=gizli';
const env = { ...process.env, NOTAI_URL: 'http://127.0.0.1:18787', NOTAI_TOKEN: 'gizli' };
const cli = (args, input) => execFileSync(require('path').resolve(__dirname,'../../target/debug/notai'), args, { env, input, encoding: 'utf8' });
const api = async (path) => (await fetch('http://127.0.0.1:18787/api' + path, { headers: { authorization: 'Bearer gizli' } })).json();
const results = []; let p;
async function step(name, fn) {
  try { await fn(); results.push('OK   ' + name); }
  catch (e) { results.push('FAIL ' + name + ' :: ' + String(e.message).split('\n')[0]); try { await p.screenshot({ path: `${D}/fail-${name.replace(/\W+/g,'_')}.png` }); } catch {} }
}
const idle = (ms = 800) => p.waitForTimeout(ms);
async function addAsset(f) {
  await p.locator('.fin-actions button', { hasText: 'Varlık ekle' }).click(); await idle(400);
  await p.locator('.sheet input[placeholder^="Örn. Kadıköy"]').fill(f.name);
  await p.getByLabel('Tür').selectOption(f.kind);
  await p.getByLabel('Değer').fill(f.value);
  if (f.qty) await p.getByLabel('Miktar').fill(f.qty);
  if (f.unit) await p.getByLabel('Birim').fill(f.unit);
  if (f.symbol) await p.locator('.sheet input[placeholder^="THYAO"]').fill(f.symbol);
  if (f.shot) await p.screenshot({ path: `${D}/${f.shot}` });
  await p.locator('.sheet button[type=submit]').click(); await idle();
}
(async () => {
  const b = await chromium.launch();
  p = await (await b.newContext({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 2 })).newPage();
  const errs = []; p.on('pageerror', e => errs.push(e.message));
  await p.goto(URL); await p.locator('.chat').first().waitFor();
  await step('Harcamalar → Varlıklar sekmesi (boş)', async () => {
    await p.locator('.chat', { hasText: 'Harcamalar' }).first().click();
    await p.locator('.switch2 button', { hasText: 'Varlıklar' }).click();
    await p.locator('.empty__title', { hasText: 'Henüz varlık yok' }).waitFor();
  });
  await step('varlık ekle: daire, altın, hisse', async () => {
    await addAsset({ name: 'Kadıköy daire', kind: 'gayrimenkul', value: '5.000.000' });
    await addAsset({ name: 'Altın birikimi', kind: 'altin', value: '40.000', qty: '12,5', unit: 'gram', symbol: 'gram altın', shot: 'asset-01-form.png' });
    await addAsset({ name: 'THY hisseleri', kind: 'hisse', value: '31.000', qty: '100', unit: 'adet', symbol: 'THYAO' });
    await p.locator('.fin-asset', { hasText: 'THY hisseleri' }).waitFor();
    const total = await p.locator('.fin-tile__val').first().innerText();
    if (!total.startsWith('5.071.000')) throw new Error(total);
    await p.screenshot({ path: `${D}/asset-02-liste.png`, fullPage: true });
  });
  await step('ajan: notai asset-price (gram altın 3.400) → konuya NotAI mesajı', async () => {
    const v = await api('/assets');
    const gold = v.assets.find(a => a.name === 'Altın birikimi');
    const out = cli(['asset-price', gold.topic_id, '3.400']);
    results.push('     ' + out);
    if (!out.includes('arttı')) throw new Error(out);
    await p.reload(); await p.locator('.chat').first().waitFor();
    const title = await p.locator('.notice__title').innerText();
    if (!title.includes('Varlık · Altın birikimi')) throw new Error(title);
  });
  await step('Varlıklar listesinden konuyu aç; menüde Varlık bilgisi', async () => {
    await p.locator('.chat', { hasText: 'Harcamalar' }).first().click();
    await p.locator('.switch2 button', { hasText: 'Varlıklar' }).click();
    await p.locator('.fin-asset__main', { hasText: 'Altın birikimi' }).click(); await idle();
    await p.locator('.bubble--notai', { hasText: 'arttı' }).first().waitFor();
    await p.screenshot({ path: `${D}/asset-03-mesaj.png` });
    await p.getByLabel('Konu menüsü').click();
    await p.locator('.sheet .setrow', { hasText: 'Varlık bilgisi' }).click(); await idle();
    const val = await p.getByLabel('Değer').inputValue();
    if (val !== '42.500,00') throw new Error('değer ' + val);
    await p.getByLabel('Değer').fill('43.000');
    await p.locator('.sheet button[type=submit]').click(); await idle();
  });
  await step('ana ekranda Varlıklar ana konusu + alt konular', async () => {
    await p.getByLabel('Geri').click(); await idle();
    if (await p.locator('.chat', { hasText: 'Varlıklar' }).count() < 1) await p.getByLabel('Geri').click();
    await p.locator('.chat', { hasText: 'Varlıklar' }).first().waitFor();
    await p.screenshot({ path: `${D}/asset-04-ana.png` });
    const v = await api('/assets');
    if (v.total !== '5.074.000,00 ₺') throw new Error(v.total);
  });
  await step('koyu tema varlık listesi', async () => {
    await p.emulateMedia({ colorScheme: 'dark' });
    await p.getByLabel('Geri').click(); await idle();
    await p.locator('.chat', { hasText: 'Harcamalar' }).first().click();
    await p.locator('.switch2 button', { hasText: 'Varlıklar' }).click(); await idle();
    await p.screenshot({ path: `${D}/asset-05-koyu.png` });
  });
  console.log(results.join('\n'));
  if (errs.length) console.log('PAGE ERRORS:', errs);
  await b.close();
})();

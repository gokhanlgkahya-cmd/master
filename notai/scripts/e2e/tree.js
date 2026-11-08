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
const idle = (ms = 700) => p.waitForTimeout(ms);
async function newTopic(name, kind) {
  await p.locator('.sheet input[type=text]').fill(name);
  if (kind) await p.locator('.sheet .seg3 button', { hasText: kind }).click();
  await p.locator('.sheet button[type=submit]').click(); await idle();
}
(async () => {
  const b = await chromium.launch();
  p = await (await b.newContext({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 2 })).newPage();
  const errs = []; p.on('pageerror', e => errs.push(e.message));
  await p.goto(URL); await p.locator('.chat').first().waitFor();

  await step('ana konu: Sabun işi', async () => {
    await p.getByLabel('Yeni konu').click(); await newTopic('Sabun işi', 'Alan');
    await p.locator('.chat', { hasText: 'Sabun işi' }).waitFor();
  });
  await step('alt konular: konu menüsü → Alt konu ekle', async () => {
    await p.locator('.chat', { hasText: 'Sabun işi' }).click();
    await p.locator('.flowbar__name', { hasText: 'Sabun işi' }).waitFor();
    await p.getByLabel('Konu menüsü').click();
    await p.locator('.sheet .setrow', { hasText: 'Alt konu ekle' }).click(); await idle(400);
    await p.locator('.sheet__title', { hasText: 'yeni alt konu' }).waitFor();
    await newTopic('Malzeme hareketleri', 'Alan');
    // Artık hub: + ile ikinci ve üçüncü alt konu
    await p.locator('.chat', { hasText: 'Sabun işi · genel' }).waitFor();
    await p.getByLabel('Alt konu ekle').click(); await newTopic('Yapılacaklar', 'Proje');
    await p.getByLabel('Alt konu ekle').click(); await newTopic('Para alışverişi', 'Alan');
    const names = await p.locator('.chat__name').allInnerTexts();
    if (names[0] !== 'Sabun işi · genel' || names.length !== 4) throw new Error(names.join(','));
  });
  await step('var olan konuyu taşı: Kostik tedarik → Sabun işi / Malzeme hareketleri', async () => {
    await p.getByLabel('Geri').click(); await p.locator('.chat').first().waitFor();
    await p.locator('.chat', { hasText: 'Kostik tedarik' }).click();
    await p.getByLabel('Konu menüsü').click();
    await p.locator('.sheet .setrow', { hasText: 'Üst konuyu değiştir' }).click();
    await p.locator('.sheet .pick .topic', { hasText: 'Sabun işi / Malzeme hareketleri' }).click(); await idle();
    await p.getByLabel('Geri').click(); await idle();
    // Geri: üst konunun listesine döner
    await p.locator('.flowbar__name', { hasText: 'Malzeme hareketleri' }).waitFor();
    await p.locator('.chat', { hasText: 'Kostik tedarik' }).waitFor();
  });
  await step('ana ekranda yalnız en üst konular; Sabun işi rozet/alt konu sayısı', async () => {
    await p.getByLabel('Geri').click(); await p.getByLabel('Geri').click(); await p.locator('.chat').first().waitFor();
    const names = await p.locator('.chat__name').allInnerTexts();
    if (names.includes('Kostik tedarik') || names.includes('Malzeme hareketleri')) throw new Error(names.join(','));
    const subs = await p.locator('.chat', { hasText: 'Sabun işi' }).locator('.chat__subs').innerText();
    if (!subs.includes('3')) throw new Error('alt konu sayısı: ' + subs);
    await p.screenshot({ path: `${D}/tree-01-ana.png` });
  });
  await step('Sabun işi açılınca: genel yazışma + alt konular', async () => {
    await p.locator('.chat', { hasText: 'Sabun işi' }).click(); await idle();
    await p.screenshot({ path: `${D}/tree-02-sabun.png` });
  });
  await step('alt konuya yaz → üst konuda ve Ana Günlük\'te de görünür', async () => {
    await p.locator('.chat', { hasText: 'Para alışverişi' }).click();
    await p.locator('.field__input').fill('Ayhan Kimya\'ya 1.850 ₺ ödendi'); await p.keyboard.press('Enter'); await idle();
    await p.getByLabel('Geri').click(); await idle();
    await p.locator('.chat', { hasText: 'Sabun işi · genel' }).click();
    await p.locator('.bubble', { hasText: '1.850 ₺ ödendi' }).waitFor();
    const meta = await p.locator('.entry', { hasText: '1.850 ₺ ödendi' }).locator('.meta__topic').innerText();
    if (!meta.includes('Para alışverişi')) throw new Error('alt konu işareti yok: ' + meta);
    await p.screenshot({ path: `${D}/tree-03-genel.png` });
  });
  await step('ana konuya yaz → AI alt konuya dağıtır (ajan)', async () => {
    await p.locator('.field__input').fill('Zeytinyağı 20 litre geldi'); await p.keyboard.press('Enter'); await idle();
    const jobs = JSON.parse(cli(['jobs', '--limit', '10']));
    const job = jobs.find(j => j.text.startsWith('Zeytinyağı 20 litre'));
    if (!job.user.includes('"Sabun işi" konusuna yazıldı')) throw new Error('istemde dağıtım notu yok');
    const mal = (job.system.match(/([0-9a-f-]{36}) · Alan · Sabun işi \/ Malzeme hareketleri\n/) || [])[1];
    if (!mal) throw new Error('istemde yol adı yok');
    cli(['answer', job.entry_id, '-'], JSON.stringify({ topic: { existing_id: mal, new_name: null, new_kind: null, parent_id: null, confidence: 0.9 },
      tasks: [], tags: [], people: [], reminder: null, question: null }));
    await p.reload(); await p.locator('.chat').first().waitFor();
    await p.locator('.chat', { hasText: 'Sabun işi' }).click(); await p.locator('.chat', { hasText: 'Sabun işi · genel' }).click();
    const chip = p.locator('.chip', { hasText: '→ Malzeme hareketleri' });
    await chip.waitFor();
    await p.screenshot({ path: `${D}/tree-04-dagitim.png` });
    await chip.locator('.chip__body').click(); await idle();
    const meta = await p.locator('.entry', { hasText: 'Zeytinyağı 20 litre' }).locator('.meta__topic').innerText();
    if (!meta.includes('Malzeme hareketleri')) throw new Error('dağıtılmadı: ' + meta);
  });
  await step('Ana Günlük: kayıtlar tek (mükerrer yok)', async () => {
    await p.getByLabel('Geri').click(); await p.getByLabel('Geri').click(); await p.locator('.chat').first().waitFor();
    await p.locator('.chat', { hasText: 'Ana Günlük' }).click(); await p.locator('.bubble').first().waitFor();
    const n = await p.locator('.bubble', { hasText: '1.850 ₺ ödendi' }).count();
    if (n !== 1) throw new Error('adet: ' + n);
  });
  console.log(results.join('\n')); console.log('pageerror: ' + errs.join(' | '));
  await b.close();
})();

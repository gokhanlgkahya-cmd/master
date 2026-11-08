const { chromium } = require('playwright');
const { execSync } = require('child_process');
const D = require('path').join(__dirname, 'out'); require('fs').mkdirSync(D, { recursive: true });
const URL = 'http://127.0.0.1:18080/?api=http://127.0.0.1:18787&token=gizli';
const results = []; let p;
async function step(name, fn) {
  try { await fn(); results.push('OK   ' + name); }
  catch (e) { results.push('FAIL ' + name + ' :: ' + String(e.message).split('\n')[0]); try { await p.screenshot({ path: `${D}/fail-${name.replace(/\W+/g,'_')}.png` }); } catch {} }
}
const idle = (ms = 800) => p.waitForTimeout(ms);
(async () => {
  const b = await chromium.launch({ args: ['--use-fake-ui-for-media-stream', '--use-fake-device-for-media-stream'] });
  const ctx = await b.newContext({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 2 });
  await ctx.grantPermissions(['microphone'], { origin: 'http://127.0.0.1:18080' });
  p = await ctx.newPage();
  const errs = []; p.on('pageerror', e => errs.push(e.message));
  await p.goto(URL); await p.locator('.chat').first().waitFor();
  await step('mikrofon → konuş → Gönder (Whisper kapalı): kayıt bekler, kaybolmaz', async () => {
    await p.getByLabel('Yeni kayıt').click(); await idle(400);
    await p.getByLabel('Sesli yakala').click();
    await p.locator('.voicestrip').waitFor();
    await idle(2500);
    await p.screenshot({ path: `${D}/voice-01-kayit.png` });
    await p.locator('.voicestrip__send').click();
    await p.locator('.bubble', { hasText: 'yazıya çevriliyor' }).waitFor({ timeout: 8000 });
    await p.screenshot({ path: `${D}/voice-02-bekliyor.png` });
  });
  await step('Whisper açıldı → sunucu sesi yazıya çevirir, metin akışa gelir', async () => {
    execSync(`bash ${__dirname}/run-stt.sh`);
    await idle(9000);
    await p.reload(); await p.locator('.chat', { hasText: 'Ana Günlük' }).click();
    await p.locator('.bubble', { hasText: 'kombi servisini ara' }).waitFor({ timeout: 10000 });
    if (await p.locator('.bubble', { hasText: 'yazıya çevriliyor' }).count()) throw new Error('yer tutucu kaldı');
    await p.screenshot({ path: `${D}/voice-03-duzeltildi.png` });
  });
  console.log(results.join('\n'));
  if (errs.length) console.log('PAGE ERRORS:', errs);
  await b.close();
})();

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
(async () => {
  const b = await chromium.launch();
  p = await (await b.newContext({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 2 })).newPage();
  const logs = []; p.on('pageerror', e => logs.push(e.message));
  await p.goto(URL); await p.locator('.chat').first().waitFor();
  let entry, topicId;
  await step('yeni kayıt: "Ayhan\'ı ara, teklif lazım"', async () => {
    await p.getByLabel('Yeni kayıt').click(); await idle(400);
    await p.keyboard.type("Ayhan'ı ara, teklif lazım"); await p.keyboard.press('Enter'); await idle();
    await p.locator('.bubble', { hasText: "Ayhan'ı ara" }).waitFor();
  });
  await step('ajan emin değil → soru sorar (notai answer)', async () => {
    const jobs = JSON.parse(cli(['jobs', '--limit', '10']));
    const job = jobs.find(j => j.text.startsWith("Ayhan'ı ara"));
    entry = job.entry_id;
    topicId = (job.system.match(/([0-9a-f-]{36})[^\n]*Kostik tedarik/) || [])[1];
    const out = cli(['answer', entry, '-'], JSON.stringify({ topic: null, tasks: [], tags: [], people: [], reminder: null,
      question: 'Hangi Ayhan? Ayhan Kimya mı, kostik teklifi için mi?' }));
    results.push('     ' + out.trim());
  });
  await step('ana ekranda "NotAI soruyor" şeridi', async () => {
    await p.getByLabel('Geri').click(); await p.locator('.chat').first().waitFor();
    await p.locator('.notice').waitFor({ timeout: 4000 });
    const t = await p.locator('.notice__title').innerText();
    if (!t.startsWith('NotAI soruyor')) throw new Error(t);
    await p.screenshot({ path: `${D}/qa-01-serit.png` });
  });
  await step('şeride dokun → Ana Günlük, soru balonu', async () => {
    await p.locator('.notice__open').click();
    await p.locator('.aimsg').waitFor();
    if (await p.locator('.notice').count()) throw new Error('şerit kapanmadı');
    await p.screenshot({ path: `${D}/qa-02-soru.png` });
  });
  await step('soruyu cevapla', async () => {
    await p.locator('.aimsg__input').fill('Evet, Ayhan Kimya. Perşembe 14:00\'te arayayım');
    await p.getByLabel('Cevabı gönder').click(); await idle(1200);
    await p.locator('.aimsg__answer', { hasText: 'Evet, Ayhan Kimya' }).waitFor();
  });
  await step('ajan cevabı görür → konu + saatli görev önerir', async () => {
    const jobs = JSON.parse(cli(['jobs', '--limit', '10']));
    const job = jobs.find(j => j.entry_id === entry);
    if (!job || !job.user.includes('<cevap>Evet, Ayhan Kimya')) throw new Error('cevap istemde yok');
    cli(['answer', entry, '-'], JSON.stringify({
      topic: { existing_id: topicId, new_name: null, new_kind: null, confidence: 0.9 },
      tasks: [{ title: 'Ayhan Kimya\'yı ara (teklif)', due_date: '2026-11-12', due_time: '14:00', priority: 'normal', confidence: 0.85 }],
      tags: [], people: [{ name: 'Ayhan Kimya', confidence: 0.8 }], reminder: null, question: null }));
    await p.reload(); await p.locator('.chat', { hasText: 'Ana Günlük' }).click();
    await p.locator('.chip', { hasText: 'Kostik tedarik' }).waitFor();
    await p.screenshot({ path: `${D}/qa-03-oneriler.png` });
  });
  await step('görev çipini kabul et → hatırlatma 13:45', async () => {
    await p.locator('.chip', { hasText: 'Görev · Perşembe 12 Kas 14:00' }).locator('.chip__body').click(); await idle();
    await p.getByLabel('Geri').click(); await p.locator('nav.tabbar button', { hasText: 'Hatırlatma' }).click(); await idle();
    const t = await p.locator('.rem', { hasText: 'Ayhan Kimya' }).locator('.rem__time').innerText();
    if (!t.includes('13:45')) throw new Error(t);
    await p.screenshot({ path: `${D}/qa-04-hatirlatma.png` });
  });
  console.log(results.join('\n')); console.log('pageerror: ' + logs.join(' | '));
  await b.close();
})();

// Drives the Forest page through `kanopy serve` in headless Chromium: plays, drags the listener,
// draws a box, opens an underwater example, follows a species; saves screenshots and fails on
// page errors. Usage: node tools/check-pages.js [http://127.0.0.1:8710] [out-dir]
const { chromium } = require('playwright');
const url = process.argv[2] || 'http://127.0.0.1:8710';
const out = process.argv[3] || 'docs/experiments';
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1200, height: 900 }, deviceScaleFactor: 1 });
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
  const checks = [];
  const check = (name, ok, detail) => { checks.push({ name, ok, detail }); };
  const frame = () => page.evaluate(async () => {
    const b = await window.kanopy.frame();
    const dv = new DataView(b);
    return { time: dv.getFloat64(4, true), version: dv.getUint32(12, true), playing: dv.getUint8(16), boxed: dv.getUint8(17),
      calling: dv.getUint32(20, true), active: dv.getUint32(24, true), lx: dv.getFloat32(40, true), ly: dv.getFloat32(44, true), n: (b.byteLength - 68) / 12 };
  });
  const toPx = async (x, y) => {
    const r = await page.locator('#scene').boundingBox();
    // same geometry as forest.js
    const padL = 10, padR = 92, padT = 8, padB = 8, W = 200, TOP = 50, BOTTOM = -3;
    const s = Math.min((r.width - padL - padR) / W, (r.height - padT - padB) / (TOP - BOTTOM));
    const y0 = padT + (r.height - padT - padB - s * (TOP - BOTTOM)) / 2;
    return { x: r.x + padL + x * s, y: r.y + y0 + (TOP - y) * s };
  };

  await page.goto(url);
  await page.waitForFunction(() => document.querySelectorAll('.chip').length > 0);
  check('species legend', (await page.locator('.chip').count()) >= 1);
  await page.click('#play');
  await wait(4000);
  let f = await frame();
  check('plays in real time', f.playing === 1 && f.time > 3 && f.time < 5, `t=${f.time.toFixed(2)}`);
  check('animals calling', f.calling > 0 && f.active > 0, `calling ${f.calling}, active ${f.active}, of ${f.n}`);
  await page.screenshot({ path: `${out}/06-forest-page.png` });

  // drag the listener up into the canopy
  const a = await toPx(100, 1.5), b = await toPx(140, 22);
  await page.mouse.move(a.x, a.y);
  await page.mouse.down();
  for (let k = 1; k <= 10; k++) { await page.mouse.move(a.x + ((b.x - a.x) * k) / 10, a.y + ((b.y - a.y) * k) / 10); await wait(30); }
  await page.mouse.up();
  await wait(2500);
  f = await frame();
  check('listener follows the drag', Math.abs(f.lx - 140) < 1.5 && Math.abs(f.ly - 22) < 1.5, `at ${f.lx.toFixed(1)}, ${f.ly.toFixed(1)}`);
  await page.screenshot({ path: `${out}/06-listener-in-canopy.png` });

  // box mode: draw a box over the floor
  await page.click('#mode button[data-v=box]');
  await wait(300);
  const c = await toPx(30, 20), d = await toPx(80, 0.2);
  await page.mouse.move(c.x, c.y);
  await page.mouse.down();
  for (let k = 1; k <= 8; k++) { await page.mouse.move(c.x + ((d.x - c.x) * k) / 8, c.y + ((d.y - c.y) * k) / 8); await wait(30); }
  await page.mouse.up();
  await wait(2500);
  f = await frame();
  check('box listening', f.boxed === 1, '');
  await page.screenshot({ path: `${out}/06-box.png` });

  // an underwater example, following a species
  await page.click('#mode button[data-v=listener]');
  const v0 = f.version;
  await page.selectOption('#examples', 'Sunken kelp');
  await page.waitForFunction((v) => window.kanopy.frame().then((b) => new DataView(b).getUint32(12, true) !== v), v0, { timeout: 5000 });
  await page.locator('.chip').nth(4).click();
  await wait(5000);
  f = await frame();
  check('example loads and keeps playing', f.playing === 1 && f.calling >= 0, `t=${f.time.toFixed(1)}`);
  check('following moved the listener', !(Math.abs(f.lx - 140) < 0.5 && Math.abs(f.ly - 22) < 0.5), `at ${f.lx.toFixed(1)}, ${f.ly.toFixed(1)}`);
  await page.screenshot({ path: `${out}/06-underwater-following.png` });

  // a knob that grows a new forest
  const before = (await frame()).version;
  const knob = page.locator('#k-forest .dial').first();
  const kb = await knob.boundingBox();
  await page.mouse.move(kb.x + kb.width / 2, kb.y + kb.height / 2);
  await page.mouse.down();
  await page.mouse.move(kb.x + kb.width / 2, kb.y - 40, { steps: 5 });
  await page.mouse.up();
  await wait(1500);
  check('density knob rebuilds', (await frame()).version !== before, '');
  await page.screenshot({ path: `${out}/06-thicker.png` });

  check('no page errors', errors.length === 0, errors.join(' | '));
  await browser.close();
  let failed = 0;
  for (const c of checks) {
    console.log(`${c.ok ? 'ok  ' : 'FAIL'} ${c.name}${c.detail ? ' · ' + c.detail : ''}`);
    failed += !c.ok;
  }
  console.log(`${checks.length - failed} of ${checks.length} page checks pass`);
  process.exit(failed ? 1 : 0);
})();

// Drives the Forest page through `kanopy serve` in headless Chromium: plays, drags the listener,
// draws a box, opens an underwater example, follows a species; saves screenshots and fails on
// page errors. Then the Species page: picks a colour, drags an area, adds from the library,
// applies, and checks the library keeps it. Then the Designer: the draft shared with the Species
// page, Call and Signature alone and over the forest, a range drag, the below-hearing flag in
// water, Speed. Usage: node tools/check-pages.js [http://127.0.0.1:8710] [out-dir] [prefix]
const { chromium } = require('playwright');
const url = process.argv[2] || 'http://127.0.0.1:8710';
const out = process.argv[3] || 'docs/experiments';
const tag = process.argv[4] || '09'; // screenshot prefix: the batch
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
  await page.screenshot({ path: `${out}/${tag}-forest-page.png` });

  // drag the listener up into the canopy
  const a = await toPx(100, 1.5), b = await toPx(140, 22);
  await page.mouse.move(a.x, a.y);
  await page.mouse.down();
  for (let k = 1; k <= 10; k++) { await page.mouse.move(a.x + ((b.x - a.x) * k) / 10, a.y + ((b.y - a.y) * k) / 10); await wait(30); }
  await page.mouse.up();
  await wait(2500);
  f = await frame();
  check('listener follows the drag', Math.abs(f.lx - 140) < 1.5 && Math.abs(f.ly - 22) < 1.5, `at ${f.lx.toFixed(1)}, ${f.ly.toFixed(1)}`);
  await page.screenshot({ path: `${out}/${tag}-listener-in-canopy.png` });

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
  await page.screenshot({ path: `${out}/${tag}-box.png` });

  // an underwater example, following a species
  await page.click('#mode button[data-v=listener]');
  const v0 = f.version;
  await page.selectOption('#examples', 'Sunken kelp');
  await page.waitForFunction((v) => window.kanopy.frame().then((b) => new DataView(b).getUint32(12, true) !== v), v0, { timeout: 5000 });
  await page.locator('.chip').nth(4).click();
  await wait(5000);
  f = await frame();
  check('example loads and keeps playing', f.playing === 1 && f.calling >= 0, `t=${f.time.toFixed(1)}`);
  // Wander with Underground: the forest file (and so Export) carries it
  await page.click('#path button[data-v=wander]');
  await page.check('#underground');
  await wait(400);
  const pf = (await page.evaluate(() => window.kanopy.info())).forest.path;
  check('export path: wander, underground', pf.kind === 'wander' && pf.underground === true, JSON.stringify(pf));
  await page.uncheck('#underground');
  await page.click('#path button[data-v=follow]');
  await wait(300);
  check('following moved the listener', !(Math.abs(f.lx - 140) < 0.5 && Math.abs(f.ly - 22) < 0.5), `at ${f.lx.toFixed(1)}, ${f.ly.toFixed(1)}`);
  await page.screenshot({ path: `${out}/${tag}-underwater-following.png` });

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
  await page.screenshot({ path: `${out}/${tag}-thicker.png` });

  // ---------- the Species page
  await page.click('#play'); // stop, so the page loads quietly
  await page.goto(`${url}/species.html`);
  await page.waitForFunction(() => document.querySelectorAll('#forest-list .row').length > 0 && document.querySelectorAll('#library-list .row').length > 0);
  const nForest = await page.locator('#forest-list .row').count();
  const nLib = await page.locator('#library-list .row').count();
  check('species page lists forest and library', nForest >= 1 && nLib >= 1, `${nForest} in forest, ${nLib} in library`);
  await page.locator('#forest-list .row').nth(1).click();
  const name = await page.inputValue('#sp-name');
  // a colour another species already has: allowed
  const taken = await page.evaluate(() => getComputedStyle(document.querySelector('#forest-list .row i')).backgroundColor);
  const swatch = await page.evaluate((rgb) => [...document.querySelectorAll('.swatch')].findIndex((b) => getComputedStyle(b).backgroundColor === rgb), taken);
  await page.locator('.swatch').nth(swatch).click();
  check('nothing live before Apply', !(await page.locator('#apply').isDisabled()) && (await page.evaluate(() => window.kanopy.info())).forest.species[1].colour !== (await page.locator('.swatch').nth(swatch).getAttribute('data-c')), '');
  // drag the area a little to the right
  const box = await page.locator('#scene').boundingBox();
  const geo = (x, y) => {
    const padL = 10, padR = 92, padT = 8, padB = 8, W = 200, TOP = 50, BOTTOM = -3;
    const s = Math.min((box.width - padL - padR) / W, (box.height - padT - padB) / (TOP - BOTTOM));
    const y0 = padT + (box.height - padT - padB - s * (TOP - BOTTOM)) / 2;
    return { x: box.x + padL + x * s, y: box.y + y0 + (TOP - y) * s };
  };
  const area = await page.evaluate(() => window.kanopy.info().then((i) => i.forest.species[1].habitat.area));
  const mid = geo((area.x0 + area.x1) / 2, (area.y0 + area.y1) / 2);
  const shift = Math.min(10, 200 - area.x1);
  const to = geo((area.x0 + area.x1) / 2 + shift, (area.y0 + area.y1) / 2);
  await page.mouse.move(mid.x, mid.y);
  await page.mouse.down();
  await page.mouse.move(to.x, to.y, { steps: 6 });
  await page.mouse.up();
  // add a library species, then apply
  await page.locator('#library-list .row').first().locator('.add').click();
  const v0s = (await frame()).version;
  await page.screenshot({ path: `${out}/${tag}-species-editing.png` });
  await page.click('#apply');
  await page.waitForFunction((v) => window.kanopy.frame().then((b) => new DataView(b).getUint32(12, true) !== v), v0s, { timeout: 8000 });
  const after = await page.evaluate(() => window.kanopy.info());
  const want = await page.locator('.swatch').nth(swatch).getAttribute('data-c');
  check('Apply grows the forest with the edits', after.forest.species.length === nForest + 1 && after.forest.species[1].colour === want, `${after.forest.species.length} species, colour ${after.forest.species[1].colour}`);
  const shared = await page.evaluate((sp) => window.palette.colours(sp), after.forest.species);
  check('two species may share a colour', shared[0] === shared[1], `${shared[0]} and ${shared[1]}`);
  const a2 = after.forest.species[1].habitat.area;
  check('area drag applied', Math.abs(a2.x0 - area.x0 - shift) < 1.5, `x0 ${area.x0} → ${a2.x0}`);
  check('Apply and Revert off after applying', await page.locator('#apply').isDisabled() && await page.locator('#revert').isDisabled(), '');
  // the library keeps every species, across reloads
  await page.reload();
  await page.waitForFunction(() => document.querySelectorAll('#library-list .row').length > 0);
  const lib = await page.evaluate(() => window.kanopy.library());
  const saved = lib.find((s) => s.name === name);
  check('library keeps the applied species', !!saved && saved.colour === want, saved ? saved.colour : 'missing');
  // revert throws edits away
  await page.locator('#forest-list .row').first().click();
  await page.click('#new');
  check('New adds a draft', (await page.locator('#forest-list .row').count()) === nForest + 2, '');
  await page.click('#revert');
  check('Revert drops the draft', (await page.locator('#forest-list .row').count()) === nForest + 1, '');
  await page.click('#play');
  await wait(2500);
  await page.screenshot({ path: `${out}/${tag}-species-page.png` });

  // ---------- the Designer
  await page.click('#play'); // stop again
  await wait(300);
  const aud = () => page.evaluate(async () => new DataView(await window.kanopy.frame()).getUint8(18));
  // an edit on the Species page, not applied, then over to the Designer
  await page.locator('#forest-list .row').nth(2).click();
  const pickName = await page.inputValue('#sp-name');
  await page.fill('#sp-count', '7');
  await page.dispatchEvent('#sp-count', 'change');
  await page.goto(`${url}/designer.html`);
  await page.waitForFunction(() => document.querySelectorAll('#forest-list .row').length > 0 && document.querySelector('#flag').textContent);
  check('Designer opens on the species picked there', (await page.inputValue('#sp-name')) === pickName, await page.inputValue('#sp-name'));
  check('the draft comes along', !(await page.locator('#apply').isDisabled()), await page.textContent('#status'));
  // drag the Pitch range's top thumb to the right
  const pitchRow = page.locator('.range').filter({ has: page.locator('.name', { hasText: /^Pitch$/ }) });
  const pitchBefore = await pitchRow.locator('.val').textContent();
  const tr = await pitchRow.locator('.track').boundingBox();
  const hiLeft = await pitchRow.locator('.thumb.hi').evaluate((e) => parseFloat(e.style.left) / 100);
  const hx = tr.x + tr.width * hiLeft, hy = tr.y + tr.height / 2;
  await page.mouse.move(hx, hy);
  await page.mouse.down();
  await page.mouse.move(Math.min(tr.x + tr.width - 2, hx + 40), hy, { steps: 6 });
  await page.mouse.up();
  const afterPitch = await pitchRow.locator('.val').textContent();
  check('a range drag changes the draft', afterPitch !== pitchBefore, `${pitchBefore} → ${afterPitch}`);
  // stopped forest: the creature alone
  const t0 = (await frame()).time;
  await page.click('#call-btn');
  await wait(250);
  check('Call plays with the forest stopped', (await aud()) === 1 && (await frame()).time === t0, `auditioning ${await aud()}`);
  await wait(400);
  await page.screenshot({ path: `${out}/${tag}-designer-call.png` });
  await wait(4000);
  check('an audition ends by itself', (await aud()) === 0, '');
  // playing forest: ducked under the signature
  await page.click('#play');
  await wait(1500);
  await page.click('#sig-btn');
  await wait(300);
  const fp = await frame();
  check('Signature plays over the forest', (await aud()) === 1 && fp.playing === 1, `playing ${fp.playing}`);
  await wait(300);
  await page.screenshot({ path: `${out}/${tag}-designer-ducked.png` });
  check('readout says ducked', (await page.textContent('#readouts')).includes('DUCKED'), await page.textContent('#readouts'));
  // back on the Species page both edits are in the draft
  await page.goto(`${url}/species.html`);
  await page.waitForFunction(() => document.querySelectorAll('#forest-list .row').length > 0);
  const both = await page.evaluate((nm) => { const d = JSON.parse(sessionStorage.getItem('kanopy.draft')); const s = d.species.find((x) => x.name === nm); return [s.habitat.individuals, s.voice.pitch]; }, pickName);
  check('one draft across both pages', both[0] === 7 && (await page.inputValue('#sp-name')) === pickName && !(await page.locator('#apply').isDisabled()), `count ${both[0]}, pitch ${both[1]}`);
  await page.click('#revert');
  // water: a high body is flagged
  await page.evaluate(() => window.kanopy.cmd({ type: 'medium', medium: 'water' }));
  await page.goto(`${url}/designer.html`);
  await page.waitForFunction(() => /water/i.test(document.querySelector('#flag').textContent));
  const swifts = await page.evaluate(() => [...document.querySelectorAll('#forest-list .row span')].findIndex((e) => e.textContent === 'Swifts'));
  if (swifts >= 0) await page.locator('#forest-list .row').nth(swifts).click();
  const bodyRow = page.locator('.range').filter({ has: page.locator('.name', { hasText: /^Body$/ }) });
  await bodyRow.locator('.track').focus();
  for (let k = 0; k < 12; k++) await page.keyboard.press('ArrowRight');
  await page.waitForFunction(() => document.querySelector('#flag').classList.contains('bad'), null, { timeout: 5000 }).catch(() => {});
  const flag = await page.textContent('#flag');
  check('below hearing is flagged in water', await page.locator('#flag.bad').count() === 1, flag);
  await page.click('#call-btn');
  await wait(500);
  await page.screenshot({ path: `${out}/${tag}-designer-water.png` });
  await page.click('#revert');
  await page.evaluate(() => window.kanopy.cmd({ type: 'medium', medium: 'earth' }));
  await wait(500);
  // Speed: its own, then set, then applied
  await page.locator('#forest-list .row').first().click();
  check('speed starts as its movement\'s own', await page.isChecked('#sp-own-speed'), await page.locator('#k-moving .val').textContent());
  await page.locator('#k-moving .dial').focus();
  await page.keyboard.press('ArrowUp');
  await page.keyboard.press('ArrowUp');
  const sp0 = await page.evaluate(() => window.kanopy.info().then((i) => i.forest.species[0].habitat.speed));
  const vs = (await frame()).version;
  await page.click('#apply');
  await page.waitForFunction((v) => window.kanopy.frame().then((b) => new DataView(b).getUint32(12, true) !== v), vs, { timeout: 8000 });
  const sp1 = await page.evaluate(() => window.kanopy.info().then((i) => i.forest.species[0].habitat.speed));
  check('Speed applies', sp0 == null && typeof sp1 === 'number' && !(await page.isChecked('#sp-own-speed')), `${sp0} → ${sp1} m/s`);
  await page.click('#play');

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

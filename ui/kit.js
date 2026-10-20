// kanopy · knobs, ranges and segmented controls.
(function () {
  /**
   * A knob. opts: name, min, max, value, def (double-click), step, fmt(v) → text, colour (CSS),
   * log (log scale), live (call onChange while dragging; otherwise only on release),
   * onChange(v).
   */
  function knob(parent, opts) {
    const el = document.createElement('div');
    el.className = 'knob';
    el.innerHTML = `<div class="dial" tabindex="0" role="slider"><div class="cap"></div><div class="ptr"></div></div><div class="name"></div><div class="val"></div>`;
    parent.appendChild(el);
    const dial = el.querySelector('.dial');
    const nameEl = el.querySelector('.name');
    const valEl = el.querySelector('.val');
    nameEl.textContent = opts.name;
    dial.setAttribute('aria-label', opts.name);
    dial.style.setProperty('--c', opts.colour || 'var(--graphite)');
    const lo = opts.log ? Math.log(opts.min) : opts.min;
    const hi = opts.log ? Math.log(opts.max) : opts.max;
    const toU = (v) => ((opts.log ? Math.log(v) : v) - lo) / (hi - lo);
    const fromU = (u) => { const x = lo + (hi - lo) * Math.min(1, Math.max(0, u)); return opts.log ? Math.exp(x) : x; };
    const snap = (v) => (opts.step ? Math.round(v / opts.step) * opts.step : v);
    let value = opts.value;
    function draw() {
      const u = toU(value);
      dial.style.setProperty('--a0', '0deg');
      dial.style.setProperty('--a1', `${u * 270}deg`);
      dial.querySelector('.ptr').style.transform = `rotate(${-135 + u * 270}deg)`;
      valEl.textContent = opts.fmt ? opts.fmt(value) : value.toFixed(2);
      dial.setAttribute('aria-valuetext', valEl.textContent);
    }
    function set(v, fire) {
      value = Math.min(opts.max, Math.max(opts.min, snap(v)));
      draw();
      if (fire) opts.onChange && opts.onChange(value);
    }
    let drag = null;
    dial.addEventListener('pointerdown', (e) => {
      dial.setPointerCapture(e.pointerId);
      drag = { y: e.clientY, u: toU(value) };
      dial.focus();
      e.preventDefault();
    });
    dial.addEventListener('pointermove', (e) => {
      if (!drag) return;
      const k = e.shiftKey ? 0.0015 : 0.006;
      set(fromU(drag.u + (drag.y - e.clientY) * k), !!opts.live);
    });
    const end = () => { if (drag) { drag = null; opts.onChange && opts.onChange(value); } };
    dial.addEventListener('pointerup', end);
    dial.addEventListener('pointercancel', end);
    dial.addEventListener('dblclick', () => set(opts.def ?? opts.value, true));
    dial.addEventListener('wheel', (e) => { e.preventDefault(); set(fromU(toU(value) - Math.sign(e.deltaY) * 0.02), true); }, { passive: false });
    dial.addEventListener('keydown', (e) => {
      const d = { ArrowUp: 1, ArrowRight: 1, ArrowDown: -1, ArrowLeft: -1 }[e.key];
      if (d) { e.preventDefault(); set(fromU(toU(value) + d * (e.shiftKey ? 0.005 : 0.03)), true); }
    });
    draw();
    return {
      el,
      get value() { return value; },
      set: (v) => { if (!drag) set(v, false); },
      enable: (on) => el.classList.toggle('off', !on),
    };
  }

  /** A segmented control: buttons with data-v. onChange(v). */
  function seg(el, onChange) {
    const buttons = [...el.querySelectorAll('button')];
    const api = {
      get value() { return (buttons.find((b) => b.classList.contains('on')) || buttons[0]).dataset.v; },
      set(v) { buttons.forEach((b) => b.classList.toggle('on', b.dataset.v === v)); },
    };
    buttons.forEach((b) => b.addEventListener('click', () => { api.set(b.dataset.v); onChange(b.dataset.v); }));
    return api;
  }

  /**
   * A range: two thumbs on one track, lo…hi. opts: name, min, max, value [lo, hi], def, step,
   * log, fmt([lo, hi]) → text, colour, onChange([lo, hi]) (on release; live while dragging if
   * opts.live). Dragging between the thumbs moves both. Thumbs on top of each other lock the
   * value; the first drag direction picks which one moves. Double-click: default.
   */
  function range(parent, opts) {
    const el = document.createElement('div');
    el.className = 'range';
    el.innerHTML = `<div class="name"></div><div class="track" tabindex="0" role="group"><div class="fill"></div><div class="thumb lo"></div><div class="thumb hi"></div></div><div class="val"></div>`;
    parent.appendChild(el);
    const track = el.querySelector('.track');
    const fill = el.querySelector('.fill');
    const [tLo, tHi] = el.querySelectorAll('.thumb');
    const valEl = el.querySelector('.val');
    el.querySelector('.name').textContent = opts.name;
    track.setAttribute('aria-label', opts.name);
    track.style.setProperty('--c', opts.colour || 'var(--graphite)');
    const lo0 = opts.log ? Math.log(opts.min) : opts.min;
    const hi0 = opts.log ? Math.log(opts.max) : opts.max;
    const toU = (v) => Math.min(1, Math.max(0, ((opts.log ? Math.log(Math.max(v, opts.min)) : v) - lo0) / (hi0 - lo0)));
    const fromU = (u) => { const x = lo0 + (hi0 - lo0) * Math.min(1, Math.max(0, u)); return opts.log ? Math.exp(x) : x; };
    const snap = (v) => {
      v = Math.min(opts.max, Math.max(opts.min, v));
      return opts.step ? Math.round(v / opts.step) * opts.step : +v.toPrecision(4);
    };
    let value = [...(opts.value || [opts.min, opts.max])];
    function draw() {
      const a = toU(value[0]), b = toU(value[1]);
      tLo.style.left = `${a * 100}%`;
      tHi.style.left = `${b * 100}%`;
      fill.style.left = `${a * 100}%`;
      fill.style.width = `${Math.max(0, b - a) * 100}%`;
      el.classList.toggle('locked', value[0] === value[1]);
      valEl.textContent = opts.fmt ? opts.fmt(value) : `${value[0]}–${value[1]}`;
      track.setAttribute('aria-valuetext', valEl.textContent);
    }
    function set(v, fire) {
      let [a, b] = [snap(v[0]), snap(v[1])];
      if (a > b) [a, b] = [b, a];
      value = [a, b];
      draw();
      if (fire && opts.onChange) opts.onChange([...value]);
    }
    const uAt = (e) => { const r = track.getBoundingClientRect(); return (e.clientX - r.left) / r.width; };
    let drag = null;
    track.addEventListener('pointerdown', (e) => {
      track.setPointerCapture(e.pointerId);
      track.focus();
      e.preventDefault();
      const u = uAt(e), a = toU(value[0]), b = toU(value[1]);
      const near = 8 / track.getBoundingClientRect().width;
      let which;
      if (Math.abs(u - a) <= near && Math.abs(u - b) <= near) which = a === b ? 'pick' : (Math.abs(u - a) < Math.abs(u - b) ? 0 : 1);
      else if (Math.abs(u - a) <= near) which = 0;
      else if (Math.abs(u - b) <= near) which = 1;
      else if (u > a && u < b) which = 'both';
      else which = u < a ? 0 : 1;
      drag = { which, u0: u, a, b };
      if (which === 0 || which === 1) { const v = [...value]; v[which] = fromU(u); set(v, !!opts.live); }
    });
    track.addEventListener('pointermove', (e) => {
      if (!drag) return;
      const u = uAt(e);
      if (drag.which === 'pick') {
        if (Math.abs(u - drag.u0) < 0.004) return;
        drag.which = u < drag.u0 ? 0 : 1;
      }
      if (drag.which === 'both') {
        const d = Math.min(1 - drag.b, Math.max(-drag.a, u - drag.u0));
        set([fromU(drag.a + d), fromU(drag.b + d)], !!opts.live);
      } else {
        const v = [...value];
        v[drag.which] = fromU(u);
        if (drag.which === 0) v[0] = Math.min(v[0], value[1]); else v[1] = Math.max(v[1], value[0]);
        set(v, !!opts.live);
      }
    });
    const end = () => { if (drag) { drag = null; opts.onChange && opts.onChange([...value]); } };
    track.addEventListener('pointerup', end);
    track.addEventListener('pointercancel', end);
    track.addEventListener('dblclick', () => set(opts.def || opts.value, true));
    track.addEventListener('keydown', (e) => {
      // arrows move the whole range, shift+arrows widen or narrow it
      const d = { ArrowRight: 1, ArrowUp: 1, ArrowLeft: -1, ArrowDown: -1 }[e.key];
      if (!d) return;
      e.preventDefault();
      const k = 0.02 * d, a = toU(value[0]), b = toU(value[1]);
      if (e.shiftKey) set([value[0], fromU(Math.max(a, b + k))], true);
      else if ((d > 0 && b < 1) || (d < 0 && a > 0)) set([fromU(a + k), fromU(b + k)], true);
    });
    draw();
    return {
      el,
      get value() { return [...value]; },
      set: (v) => { if (!drag) set(v, false); },
      enable: (on) => el.classList.toggle('off', !on),
    };
  }

  window.kit = { knob, seg, range };
})();

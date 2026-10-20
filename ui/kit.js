// kanopy · knobs and segmented controls.
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

  window.kit = { knob, seg };
})();

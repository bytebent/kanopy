// kanopy · the species draft, shared by the Species and Designer pages. Edits wait here until
// Apply; moving between the pages (or reloading) keeps them. A draft only counts while the
// forest's species are still the ones it started from: load another forest and it's dropped.
(function () {
  const KEY = 'kanopy.draft';
  const SEL = 'kanopy.selected';
  const clone = (x) => JSON.parse(JSON.stringify(x));
  const get = (k) => { try { return JSON.parse(sessionStorage.getItem(k)); } catch (e) { return null; } };
  const put = (k, v) => { try { sessionStorage.setItem(k, JSON.stringify(v)); } catch (e) { /* no storage: page only */ } };
  const drop = (k) => { try { sessionStorage.removeItem(k); } catch (e) { /* nothing */ } };

  window.drafts = {
    /** The draft for this forest: { species, dirty }. */
    load(info) {
      const base = JSON.stringify(info.forest.species);
      const d = get(KEY);
      if (d && d.base === base && Array.isArray(d.species) && d.species.length) return { species: d.species, dirty: true };
      if (d) drop(KEY);
      return { species: clone(info.forest.species), dirty: false };
    },
    /** Keep edits made against these forest species. */
    save(info, species) { put(KEY, { base: JSON.stringify(info.forest.species), species }); },
    /** Applied or reverted. */
    clear() { drop(KEY); },
    /** The selected species, by name (so it survives reordering), else by index. */
    selected(species) {
      const s = get(SEL);
      if (!s) return 0;
      const i = species.findIndex((x) => x.name === s.name);
      return i >= 0 ? i : Math.min(Math.max(0, s.index | 0), species.length - 1);
    },
    select(species, i) { if (species[i]) put(SEL, { name: species[i].name, index: i }); },
  };
})();

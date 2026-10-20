// kanopy · how the page talks to the engine: through the app (Tauri) or, in a browser, through
// `kanopy serve` over HTTP. Same calls either way.
(function () {
  const T = window.__TAURI__;
  const invoke = T && T.core ? T.core.invoke : null;
  const get = (p) => fetch(p).then((r) => { if (!r.ok) throw new Error(r.statusText); return r; });
  const post = (p, body) => fetch(p, { method: 'POST', body: JSON.stringify(body ?? null) }).then((r) => r.text().then((t) => {
    if (!r.ok) throw new Error(t || r.statusText);
    return t ? JSON.parse(t) : null;
  }));
  window.kanopy = {
    inApp: !!invoke,
    info: () => invoke ? invoke('info').then(JSON.parse) : get('/api/info').then((r) => r.json()),
    frame: () => invoke ? invoke('frame') : get('/api/frame').then((r) => r.arrayBuffer()),
    cmd: (cmd) => invoke ? invoke('command', { cmd }) : post('/api/command', cmd),
    error: () => invoke ? invoke('error') : post('/api/error'),
    examples: () => invoke ? invoke('examples') : get('/api/examples').then((r) => r.json()),
    openExample: (name) => invoke ? invoke('open_example', { name }) : post('/api/open_example', { name }),
    // in a browser there's no file dialog: ask for a path on this computer instead
    open: () => {
      if (invoke) return invoke('open_forest');
      const path = prompt('Forest file or kanopy WAV to open (path on this computer):');
      return path ? post('/api/open', { path }) : Promise.resolve(null);
    },
    save: () => {
      if (invoke) return invoke('save_forest');
      const path = prompt('Save the forest as (path on this computer):', 'forest.kanopy-forest.json');
      return path ? post('/api/save', { path }) : Promise.resolve(null);
    },
    exportWav: (minutes) => {
      if (invoke) return invoke('export', { minutes });
      const path = prompt('Render to (path on this computer):', 'renders/forest.wav');
      return path ? post('/api/export', { path, minutes }) : Promise.resolve(null);
    },
    library: () => invoke ? invoke('library') : get('/api/library').then((r) => r.json()),
    librarySave: (species) => invoke ? invoke('library_save', { species }) : post('/api/library_save', { species }),
    libraryDelete: (name) => invoke ? invoke('library_delete', { name }) : post('/api/library_delete', { name }),
    libraryImport: () => {
      if (invoke) return invoke('library_import');
      const path = prompt('Species file to add to the library (path on this computer):');
      return path ? post('/api/library_import', { path }) : Promise.resolve(null);
    },
    speciesExport: (species) => {
      if (invoke) return invoke('species_export', { species });
      const path = prompt('Save the species as (path on this computer):', `${species.name}.kanopy-species.json`);
      return path ? post('/api/species_export', { path, species }) : Promise.resolve(null);
    },
    audition: (species, take) => invoke ? invoke('audition', { species, take }) : post('/api/audition', { species, take }),
    check: (species) => invoke ? invoke('check', { species }) : post('/api/check', { species }),
    exportState: () => invoke ? invoke('export_state') : get('/api/export_state').then((r) => r.json()),
  };
})();

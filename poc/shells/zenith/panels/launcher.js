let _allApps = [];
let _selIdx = 0;

function renderLauncher(s) {
  if (s.popup !== 'launcher') return;
  const el = document.getElementById('panel-launcher');
  if (!el) return;

  if (!el.dataset.built) {
    el.dataset.built = '1';
    el.innerHTML = `
      <input class="search-input" id="search" placeholder="Search applications..."
        oninput="filterApps()" autocomplete="off" spellcheck="false">
      <div class="app-list" id="app-list"></div>
    `;
  }

  _allApps = s.apps || [];
  const input = document.getElementById('search');
  if (input && !input.value) filterApps();
  setTimeout(() => { if (input) input.focus(); }, 50);
}

function filterApps() {
  const q = document.getElementById('search').value.toLowerCase();
  const list = document.getElementById('app-list');
  if (!list) return;
  list.innerHTML = '';
  const filtered = _allApps.filter(a => a.name.toLowerCase().includes(q)).slice(0, 20);
  _selIdx = 0;

  if (!filtered.length) {
    list.innerHTML = '<div class="empty-state">No matches</div>';
    return;
  }

  filtered.forEach((app, i) => {
    const div = document.createElement('div');
    div.className = 'app-item' + (i === 0 ? ' selected' : '');
    if (app.icon) {
      const img = document.createElement('img');
      img.src = app.icon;
      img.draggable = false;
      img.onerror = function () {
        const fb = document.createElement('span');
        fb.className = 'app-fallback';
        fb.textContent = app.name.charAt(0);
        this.replaceWith(fb);
      };
      div.appendChild(img);
    } else {
      const fb = document.createElement('span');
      fb.className = 'app-fallback';
      fb.textContent = app.name.charAt(0);
      div.appendChild(fb);
    }
    const name = document.createElement('span');
    name.className = 'app-name';
    name.textContent = app.name;
    div.appendChild(name);
    div.onclick = () => send({ cmd: 'launch', data: app.exec });
    list.appendChild(div);
  });
}

function moveSelection(d) {
  const items = document.querySelectorAll('.app-item');
  if (!items.length) return;
  items[_selIdx]?.classList.remove('selected');
  _selIdx = Math.max(0, Math.min(items.length - 1, _selIdx + d));
  items[_selIdx]?.classList.add('selected');
  items[_selIdx]?.scrollIntoView({ block: 'nearest' });
}

document.addEventListener('keydown', e => {
  const p = document.querySelector('.panel.active');
  if (!p) return;
  if (p.id === 'panel-launcher') {
    if (e.key === 'ArrowDown') { e.preventDefault(); moveSelection(1); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); moveSelection(-1); }
    else if (e.key === 'Tab') { e.preventDefault(); moveSelection(e.shiftKey ? -1 : 1); }
    else if (e.key === 'Enter') { e.preventDefault(); const sel = document.querySelector('.app-item.selected'); if (sel) sel.click(); }
  }
});
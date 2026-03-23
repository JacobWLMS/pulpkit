let _trayKey = '';
function renderTray(s) {
  const el = document.getElementById('tray');
  const div = document.getElementById('tray-divider');
  if (!el) return;
  const items = s.tray_items || [];
  const key = items.map(t => t.id).join(',');
  if (_trayKey === key) return;
  _trayKey = key;

  if (div) div.style.display = items.length ? '' : 'none';
  el.innerHTML = '';
  items.forEach(t => {
    const wrap = document.createElement('span');
    wrap.className = 'tray-item';
    wrap.title = t.title || t.id;
    wrap.onclick = () => send({ cmd: 'tray_activate', data: { address: t.address, click: 'left' } });
    wrap.oncontextmenu = (e) => { e.preventDefault(); send({ cmd: 'tray_activate', data: { address: t.address, click: 'right' } }); };
    if (t.icon) {
      const img = document.createElement('img');
      img.src = t.icon;
      img.draggable = false;
      wrap.appendChild(img);
    }
    el.appendChild(wrap);
  });
}
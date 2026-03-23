let _tbKey = '';
function renderTaskbar(s) {
  const el = document.getElementById('taskbar');
  if (!el) return;
  const key = (s.windows || []).map(w => w.id + (w.focused ? 'f' : '')).join(',');
  if (_tbKey === key) return;
  _tbKey = key;
  el.innerHTML = '';
  (s.windows || []).forEach(w => {
    const span = document.createElement('span');
    span.className = 'task' + (w.focused ? ' focused' : '');
    span.title = w.title;
    if (w.icon) {
      const img = document.createElement('img');
      img.src = w.icon;
      img.draggable = false;
      span.appendChild(img);
    } else {
      const fb = document.createElement('span');
      fb.className = 'fallback';
      fb.textContent = (w.app_id.split('.').pop() || '?')[0].toUpperCase();
      span.appendChild(fb);
    }
    span.onclick = () => send({ cmd: 'focus_window', data: w.id });
    el.appendChild(span);
  });
}
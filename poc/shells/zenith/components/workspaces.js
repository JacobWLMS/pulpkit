let _wsKey = '';
function renderWorkspaces(s) {
  const el = document.getElementById('workspaces');
  if (!el) return;
  const key = s.ws.map(w => w.idx + (w.active ? 'a' : '')).join(',');
  if (_wsKey === key) return;
  _wsKey = key;
  el.innerHTML = '';
  s.ws.forEach(w => {
    const span = document.createElement('span');
    span.className = 'ws' + (w.active ? ' active' : '');
    span.textContent = w.idx;
    span.onclick = () => send({ cmd: 'ws_go', data: w.idx });
    el.appendChild(span);
  });
}
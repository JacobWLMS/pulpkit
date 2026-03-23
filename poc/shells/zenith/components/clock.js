const _days = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
let _lastClock = '';

function renderClock() {
  const el = document.getElementById('clock');
  if (!el) return;
  const d = new Date();
  const h = String(d.getHours()).padStart(2, '0');
  const m = String(d.getMinutes()).padStart(2, '0');
  const text = _days[d.getDay()] + ' ' + h + ':' + m;
  if (_lastClock !== text) {
    el.textContent = text;
    _lastClock = text;
  }
}

renderClock();
setInterval(renderClock, 1000);
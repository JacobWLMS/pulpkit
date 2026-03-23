function renderStatus(s) {
  const el = document.getElementById('sysmon');

  function barColor(pct) {
    if (pct > 80) return 'var(--grv-red)';
    if (pct > 50) return 'var(--grv-yellow)';
    return 'var(--grv-aqua)';
  }

  el.innerHTML =
    '<div class="sys-item">' +
      '<span class="sys-icon" style="color:' + barColor(s.cpu) + '">' + I.cpu + '</span>' +
      '<div class="mini-bar"><div class="mini-fill" style="width:' + s.cpu + '%;background:' + barColor(s.cpu) + '"></div></div>' +
      '<span class="sys-val">' + s.cpu + '%</span>' +
    '</div>' +
    '<span class="sys-sep">&middot;</span>' +
    '<div class="sys-item">' +
      '<span class="sys-icon" style="color:' + barColor(s.mem) + '">' + I.mem + '</span>' +
      '<div class="mini-bar"><div class="mini-fill" style="width:' + s.mem + '%;background:' + barColor(s.mem) + '"></div></div>' +
      '<span class="sys-val">' + s.mem + '%</span>' +
    '</div>';
}

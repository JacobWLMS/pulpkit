function renderPower(s) {
  var el = document.getElementById('panel-power');
  if (!el.classList.contains('active')) return;

  var h = '<div class="panel-header"><span class="panel-title">Power</span>';
  h += '<div class="panel-close" onclick="send({cmd:\'dismiss\'})">&#x2715;</div></div>';

  h += '<div class="user-card"><div class="user-avatar">' + I.power + '</div>';
  h += '<div class="user-info"><span class="user-name">' + s.user + '@' + s.host + '</span>';
  h += '<span class="user-host">' + (s.uptime ? 'Up ' + s.uptime : s.kernel) + '</span></div></div>';

  h += '<div class="power-grid">';
  h += '<div class="power-btn" onclick="send({cmd:\'power_lock\'})"><span class="power-icon" style="color:var(--grv-blue)">' + I.lock + '</span><span class="power-label">Lock</span></div>';
  h += '<div class="power-btn" onclick="send({cmd:\'power_suspend\'})"><span class="power-icon" style="color:var(--grv-purple)">' + I.suspend + '</span><span class="power-label">Suspend</span></div>';
  h += '<div class="power-btn" onclick="send({cmd:\'power_logout\'})"><span class="power-icon" style="color:var(--grv-yellow)">' + I.logout + '</span><span class="power-label">Logout</span></div>';
  h += '<div class="power-btn danger" onclick="send({cmd:\'power_reboot\'})"><span class="power-icon" style="color:var(--grv-orange)">' + I.reboot + '</span><span class="power-label">Reboot</span></div>';
  h += '<div class="power-btn danger" onclick="send({cmd:\'power_shutdown\'})"><span class="power-icon" style="color:var(--grv-red)">' + I.shutdown + '</span><span class="power-label">Shutdown</span></div>';
  h += '</div>';

  el.innerHTML = h;
}

function renderPower(s) {
  if (s.popup !== 'power') return;
  const el = document.getElementById('panel-power');
  if (!el) return;

  if (!el.dataset.built) {
    el.dataset.built = '1';
    el.innerHTML = `
      <div class="panel-title">Session</div>
      <div class="power-info">
        <div class="power-host" id="pw-host"></div>
        <div class="power-detail" id="pw-kernel"></div>
        <div class="power-detail" id="pw-uptime"></div>
      </div>
      <div class="sep"></div>
      <div class="menu-item" onclick="send({cmd:'power_lock'})">
        <span class="menu-icon">${ICONS.lock}</span>
        <span class="menu-text">Lock Screen</span>
      </div>
      <div class="menu-item" onclick="send({cmd:'power_suspend'})">
        <span class="menu-icon">${ICONS.suspend}</span>
        <span class="menu-text">Suspend</span>
      </div>
      <div class="sep"></div>
      <div class="menu-item" onclick="send({cmd:'power_reboot'})">
        <span class="menu-icon">${ICONS.reboot}</span>
        <span class="menu-text">Restart</span>
      </div>
      <div class="menu-item" onclick="send({cmd:'power_shutdown'})">
        <span class="menu-icon">${ICONS.shutdown}</span>
        <span class="menu-text">Shut Down</span>
      </div>
      <div class="sep"></div>
      <div class="menu-item danger" onclick="send({cmd:'power_logout'})">
        <span class="menu-icon">${ICONS.logout}</span>
        <span class="menu-text">Log Out</span>
      </div>
    `;
  }

  const host = document.getElementById('pw-host');
  if (host) host.textContent = (s.user || '') + '@' + (s.host || '');
  const kern = document.getElementById('pw-kernel');
  if (kern) kern.textContent = s.kernel || '';
  const up = document.getElementById('pw-uptime');
  if (up) up.textContent = s.uptime ? 'up ' + s.uptime : '';
}
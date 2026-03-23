function renderSettings(s) {
  var el = document.getElementById('panel-settings');
  if (!el.classList.contains('active')) return;

  var volIcon = s.muted ? I.vol_x : (s.vol > 60 ? I.vol_hi : s.vol > 30 ? I.vol_md : I.vol_lo);
  var batIcon = s.bat_status === 'Charging' ? I.bat_c :
    (s.bat > 80 ? I.bat_f : s.bat > 60 ? I.bat_8 : s.bat > 40 ? I.bat_5 : s.bat > 20 ? I.bat_2 : I.bat_1);
  var batColor = s.bat <= 20 ? 'var(--grv-red)' : s.bat <= 40 ? 'var(--grv-yellow)' : 'var(--grv-green)';

  var h = '';
  h += '<div class="panel-header"><span class="panel-title">Quick Settings</span>';
  h += '<div class="panel-close" onclick="send({cmd:\'dismiss\'})">&#x2715;</div></div>';

  h += '<div class="user-card"><div class="user-avatar">' + I.arch + '</div>';
  h += '<div class="user-info"><span class="user-name">' + s.user + '@' + s.host + '</span>';
  h += '<span class="user-host">' + s.kernel + '</span></div></div>';

  h += '<div class="section-label">Volume</div>';
  h += '<div class="slider-row">';
  h += '<span class="slider-icon" style="color:var(--grv-blue);cursor:pointer" onclick="send({cmd:\'vol_mute\'})">' + volIcon + '</span>';
  h += '<div class="slider-track" data-type="vol"><div class="slider-fill" style="width:' + s.vol + '%;background:var(--grv-blue)"></div></div>';
  h += '<span class="slider-val">' + s.vol + '%</span></div>';
  h += '<div class="info-row"><span class="info-label">Output</span>';
  h += '<span class="info-value" style="max-width:200px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">' + s.audio_device + '</span></div>';

  h += '<div class="section-label">Brightness</div>';
  h += '<div class="slider-row">';
  h += '<span class="slider-icon" style="color:var(--grv-yellow)">' + I.bri + '</span>';
  h += '<div class="slider-track" data-type="bri"><div class="slider-fill" style="width:' + s.bright + '%;background:var(--grv-yellow)"></div></div>';
  h += '<span class="slider-val">' + s.bright + '%</span></div>';

  h += '<div class="section-label">Network</div>';
  h += '<div class="info-row" style="cursor:pointer" onclick="send({cmd:\'popup\',data:\'wifi\'})">';
  h += '<span class="info-label">' + I.wf_4 + '  Wi-Fi</span>';
  h += '<span class="info-value" style="color:var(--grv-orange)">' + (s.wifi || 'Disconnected') + ' &#x203A;</span></div>';

  h += '<div class="section-label">Quick Toggles</div>';
  h += '<div class="toggle-grid">';
  h += '<div class="toggle-btn" onclick="send({cmd:\'toggle_night\'})"><span class="toggle-icon" style="color:var(--grv-yellow)">' + I.night + '</span><span class="toggle-label">Night</span></div>';
  h += '<div class="toggle-btn" onclick="send({cmd:\'toggle_bt\'})"><span class="toggle-icon" style="color:var(--grv-blue)">' + I.bt + '</span><span class="toggle-label">Bluetooth</span></div>';
  h += '<div class="toggle-btn ' + (s.dnd ? 'on' : '') + '" onclick="send({cmd:\'toggle_dnd\'})"><span class="toggle-icon" style="color:var(--grv-red)">' + I.dnd + '</span><span class="toggle-label">DND</span></div>';
  h += '<div class="toggle-btn" onclick="send({cmd:\'screenshot\'})"><span class="toggle-icon" style="color:var(--grv-aqua)">' + I.gear + '</span><span class="toggle-label">Shot</span></div>';
  h += '<div class="toggle-btn" onclick="send({cmd:\'screenshot_full\'})"><span class="toggle-icon" style="color:var(--grv-aqua)">' + I.search + '</span><span class="toggle-label">Full</span></div>';
  h += '<div class="toggle-btn" onclick="send({cmd:\'notif_dismiss\'})"><span class="toggle-icon" style="color:var(--grv-purple)">' + I.check + '</span><span class="toggle-label">Clear</span></div>';
  h += '</div>';

  h += '<div class="section-label">Power Profile</div>';
  h += '<div class="profile-row">';
  h += '<div class="profile-btn ' + (s.power_profile === 'power-saver' ? 'active' : '') + '" onclick="send({cmd:\'set_profile\',data:\'power-saver\'})">Saver</div>';
  h += '<div class="profile-btn ' + (s.power_profile === 'balanced' ? 'active' : '') + '" onclick="send({cmd:\'set_profile\',data:\'balanced\'})">Balanced</div>';
  h += '<div class="profile-btn ' + (s.power_profile === 'performance' ? 'active' : '') + '" onclick="send({cmd:\'set_profile\',data:\'performance\'})">Turbo</div>';
  h += '</div>';

  if (s.has_bat) {
    h += '<div class="section-label">Battery</div>';
    h += '<div class="slider-row">';
    h += '<span class="slider-icon" style="color:' + batColor + '">' + batIcon + '</span>';
    h += '<div class="slider-track" style="cursor:default"><div class="slider-fill" style="width:' + s.bat + '%;background:' + batColor + '"></div></div>';
    h += '<span class="slider-val">' + s.bat + '% ' + (s.bat_status === 'Charging' ? '&#x26A1;' : '') + '</span></div>';
  }

  h += '<div class="section-label">System</div>';
  h += '<div class="info-row"><span class="info-label">CPU</span><span class="info-value">' + s.cpu + '%</span></div>';
  h += '<div class="info-row"><span class="info-label">Memory</span><span class="info-value">' + s.mem + '%</span></div>';
  h += '<div class="info-row"><span class="info-label">Disk</span><span class="info-value">' + s.disk_used + ' / ' + s.disk_total + '</span></div>';

  el.innerHTML = h;
}

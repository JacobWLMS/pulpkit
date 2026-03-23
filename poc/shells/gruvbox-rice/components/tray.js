function renderTray(s) {
  const el = document.getElementById('controls');

  const volIcon = s.muted ? I.vol_x : (s.vol > 60 ? I.vol_hi : s.vol > 30 ? I.vol_md : I.vol_lo);
  const wifiIcon = s.wifi ? (s.net_signal > 70 ? I.wf_4 : s.net_signal > 40 ? I.wf_3 : I.wf_2) : I.wf_x;
  const batIcon = s.bat_status === 'Charging' ? I.bat_c :
    (s.bat > 80 ? I.bat_f : s.bat > 60 ? I.bat_8 : s.bat > 40 ? I.bat_5 : s.bat > 20 ? I.bat_2 : I.bat_1);
  const batColor = s.bat <= 20 ? 'var(--grv-red)' : s.bat <= 40 ? 'var(--grv-yellow)' : 'var(--grv-fg-dim)';

  let html = '';

  // Tray icons
  if (s.tray_items && s.tray_items.length > 0) {
    s.tray_items.forEach(function(t) {
      var addr = t.address.replace(/'/g, "\\'");
      html += '<img class="tray-icon" src="' + escapeHtml(t.icon) + '" title="' + escapeHtml(t.title) + '"'
        + ' onclick="send({cmd:\'tray_activate\',data:{address:\'' + addr + '\',click:\'left\'}})">';
    });
    html += '<div class="tray-sep"></div>';
  }

  // Volume
  html += '<div class="ctrl-btn" onclick="send({cmd:\'popup\',data:\'settings\'})" title="Volume ' + s.vol + '%">'
    + '<span class="ctrl-icon" style="color:var(--grv-blue)">' + volIcon + '</span>'
    + '<span class="ctrl-val">' + s.vol + '</span></div>';

  // WiFi
  html += '<div class="ctrl-btn" onclick="send({cmd:\'popup\',data:\'wifi\'})" title="' + escapeHtml(s.wifi || 'Disconnected') + '">'
    + '<span class="ctrl-icon" style="color:var(--grv-aqua)">' + wifiIcon + '</span></div>';

  // Battery
  if (s.has_bat) {
    html += '<div class="ctrl-btn" title="Battery ' + s.bat + '%">'
      + '<span class="ctrl-icon" style="color:' + batColor + '">' + batIcon + '</span>'
      + '<span class="ctrl-val">' + s.bat + '%</span></div>';
  }

  // Power
  html += '<div class="ctrl-btn" onclick="send({cmd:\'popup\',data:\'power\'})" title="Power">'
    + '<span class="ctrl-icon" style="color:var(--grv-red)">' + I.power + '</span></div>';

  el.innerHTML = html;
}

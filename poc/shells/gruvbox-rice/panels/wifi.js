function renderWifi(s) {
  var el = document.getElementById('panel-wifi');
  if (!el.classList.contains('active')) return;

  var h = '<div class="panel-header">';
  h += '<div class="panel-back" onclick="send({cmd:\'popup\',data:\'settings\'})">&lt;</div>';
  h += '<span class="panel-title">Wi-Fi</span>';
  h += '<div class="panel-close" onclick="send({cmd:\'dismiss\'})">&#x2715;</div></div>';

  if (s.wifi) {
    var sigIcon = s.net_signal > 70 ? I.wf_4 : s.net_signal > 40 ? I.wf_3 : I.wf_2;
    h += '<div class="wifi-connected">';
    h += '<span class="wifi-signal" style="color:var(--grv-green)">' + sigIcon + '</span>';
    h += '<span class="wifi-ssid" style="font-weight:600">' + s.wifi + '</span>';
    h += '<span style="font-size:10px;color:var(--grv-fg-dark)">' + s.net_signal + '%</span>';
    h += '<span class="wifi-check" style="color:var(--grv-green)">' + I.check + '</span>';
    h += '</div>';
  }

  h += '<div class="section-label">Available Networks</div><div class="wifi-list">';

  var nets = s.wifi_nets || [];
  if (nets.length === 0) {
    h += '<div style="color:var(--grv-fg-dark);font-size:12px;padding:20px;text-align:center">Scanning for networks...</div>';
  } else {
    nets.forEach(function(n) {
      if (n.active) return;
      var sigIcon = n.signal > 70 ? I.wf_4 : n.signal > 40 ? I.wf_3 : n.signal > 20 ? I.wf_2 : I.wf_1;
      var sigColor = n.signal > 70 ? 'var(--grv-green)' : n.signal > 40 ? 'var(--grv-yellow)' : 'var(--grv-red)';
      var ssidSafe = n.ssid.replace(/'/g, "\\'").replace(/"/g, '&quot;');
      h += '<div class="wifi-net" onclick="send({cmd:\'wifi_con\',data:\'' + ssidSafe + '\'})">';
      h += '<span class="wifi-signal" style="color:' + sigColor + '">' + sigIcon + '</span>';
      h += '<span class="wifi-ssid">' + n.ssid + '</span>';
      if (n.secure) h += '<span class="wifi-lock">' + I.lock + '</span>';
      h += '<span style="font-size:10px;color:var(--grv-fg-dark)">' + n.signal + '%</span>';
      h += '</div>';
    });
  }
  h += '</div>';

  if (s.wifi) {
    h += '<div class="disconnect-row"><div class="disconnect-btn" onclick="send({cmd:\'wifi_dis\'})">';
    h += '<span style="color:var(--grv-red)">' + I.wf_x + '</span> Disconnect</div></div>';
  }

  el.innerHTML = h;
}

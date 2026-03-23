function renderWifi(s) {
  if (s.popup !== 'wifi') return;
  const el = document.getElementById('panel-wifi');
  if (!el) return;

  let html = '<div class="panel-title">' +
    '<span class="panel-title-icon">' + ICONS.wifi_4 + '</span>' +
    '<span>Networks</span>' +
    '<span class="flex"></span>' +
    '<span class="panel-subtitle">' + escapeHtml(s.wifi || 'Disconnected') + '</span>' +
    '</div>';

  const nets = s.wifi_nets || [];
  if (nets.length) {
    nets.slice(0, 12).forEach(net => {
      const connected = net.ssid === s.wifi;
      const safe = escapeHtml(net.ssid);
      html += '<div class="wifi-item' + (connected ? ' connected' : '') + '" onclick="send(' +
        (connected ? "{cmd:\'wifi_dis\'}" : "{cmd:\'wifi_con\',data:\'" + safe.replace(/'/g, "\\'") + "\'}") + ')">' +
        '<span class="wifi-icon">' + wifiIcon(net.signal) + '</span>' +
        '<span class="wifi-ssid">' + safe + (net.secure ? ' ' + ICONS.lock : '') + '</span>' +
        '<span class="wifi-meta">' + (connected ? 'Connected' : net.signal + '%') + '</span>' +
        '</div>';
    });
  } else {
    html += '<div class="empty-state">Scanning for networks...</div>';
  }

  el.innerHTML = html;
}
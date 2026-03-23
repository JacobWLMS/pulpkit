function renderStatus(s) {
  // Network
  const netIcon = document.getElementById('net-icon');
  if (netIcon) netIcon.textContent = s.wifi ? wifiIcon(s.net_signal) : ICONS.wifi_off;
  const netStat = document.getElementById('net-stat');
  if (netStat) netStat.className = 'stat' + (s.wifi ? '' : ' warn');

  // Volume
  const volI = document.getElementById('vol-icon');
  if (volI) volI.textContent = volIcon(s.vol, s.muted);
  setText('vol-val', s.muted ? '' : String(s.vol));
  const volStat = document.getElementById('vol-stat');
  if (volStat) volStat.className = 'stat' + (s.muted ? ' warn' : '');

  // Battery
  const batStat = document.getElementById('bat-stat');
  if (s.has_bat) {
    setText('bat-val', s.bat + '%');
    if (batStat) {
      batStat.style.display = '';
      batStat.className = 'stat' + (s.bat <= 15 ? ' crit' : s.bat <= 30 ? ' warn' : '');
    }
  } else {
    if (batStat) batStat.style.display = 'none';
  }
}
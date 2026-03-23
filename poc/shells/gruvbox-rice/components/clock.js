function renderClock(s) {
  const el = document.getElementById('clock');
  const now = new Date();
  const h = String(now.getHours()).padStart(2, '0');
  const m = String(now.getMinutes()).padStart(2, '0');
  const days = ['Sun','Mon','Tue','Wed','Thu','Fri','Sat'];
  const months = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
  const date = days[now.getDay()] + ' ' + months[now.getMonth()] + ' ' + now.getDate();

  el.innerHTML = '<div class="clock-stack" onclick="send({cmd:\'popup\',data:\'launcher\'})" style="cursor:pointer" title="App Launcher">'
    + '<span class="clock-time">' + h + ':' + m + '</span>'
    + '<span class="clock-date">' + date + '</span></div>';
}

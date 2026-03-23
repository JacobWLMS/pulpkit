function renderTaskbar(s) {
  const el = document.getElementById('taskbar');
  const wins = s.windows || [];

  if (wins.length === 0) {
    el.innerHTML = '<span class="no-win">~ no windows ~</span>';
    return;
  }

  let html = '';
  wins.forEach(w => {
    const cls = w.focused ? 'task-app focused' : 'task-app';
    const icon = w.icon ? '<img src="' + escapeHtml(w.icon) + '" onerror="this.style.display=\'none\'">' : '';
    const name = escapeHtml(w.app_id || 'unknown');
    html += '<div class="' + cls + '" onclick="send({cmd:\'focus_window\',data:' + w.id + '})" title="' + escapeHtml(w.title||'').replace(/"/g,'&quot;') + '">'
      + icon + '<span class="task-label">' + name + '</span></div>';
  });
  el.innerHTML = html;
}

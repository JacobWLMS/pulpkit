function renderWorkspaces(s) {
  const el = document.getElementById('workspaces');
  const ws = s.ws || [];

  let html = '<span class="distro">' + I.arch + '</span>';
  ws.forEach(w => {
    const cls = w.active ? 'ws-dot active' : 'ws-dot';
    html += '<div class="' + cls + '" onclick="send({cmd:\'ws_go\',data:' + w.idx + '})" title="Workspace ' + w.idx + '"></div>';
  });
  el.innerHTML = html;
}

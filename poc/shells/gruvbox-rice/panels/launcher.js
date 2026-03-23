function renderLauncher(s) {
  var el = document.getElementById('panel-launcher');
  if (!el.classList.contains('active')) {
    _launcherQuery = '';
    return;
  }

  var apps = s.apps || [];
  var q = _launcherQuery.toLowerCase();
  var filtered = q ? apps.filter(function(a) {
    return a.name.toLowerCase().indexOf(q) >= 0 || (a.exec && a.exec.toLowerCase().indexOf(q) >= 0);
  }) : apps;
  window._firstMatch = filtered.length > 0 ? filtered[0].exec : null;

  var h = '<div class="panel-header"><span class="panel-title">Applications</span>';
  h += '<div class="panel-close" onclick="send({cmd:\'dismiss\'})">&#x2715;</div></div>';

  h += '<div class="launcher-search">';
  h += '<span class="launcher-search-icon">' + I.search + '</span>';
  h += '<input class="launcher-input" type="text" placeholder="Search apps..."';
  h += ' value="' + _launcherQuery.replace(/"/g,'&quot;') + '"';
  h += ' oninput="_launcherQuery=this.value;renderLauncher(_st)"';
  h += ' onkeydown="if(event.key===\'Enter\'&&window._firstMatch){send({cmd:\'launch\',data:window._firstMatch});send({cmd:\'dismiss\'})}"';
  h += '></div>';

  h += '<div class="launcher-grid">';

  filtered.slice(0, 48).forEach(function(a) {
    var execSafe = a.exec.replace(/'/g, "\\'");
    var icon = a.icon
      ? '<img src="' + a.icon + '" onerror="this.outerHTML=\'<div class=app-placeholder>?</div>\'">'
      : '<div class="app-placeholder">?</div>';
    h += '<div class="launcher-app" onclick="send({cmd:\'launch\',data:\'' + execSafe + '\'});send({cmd:\'dismiss\'})" title="' + a.name.replace(/"/g,'&quot;') + '">';
    h += icon + '<span class="app-name">' + a.name + '</span></div>';
  });

  if (filtered.length === 0 && q) {
    h += '<div style="grid-column:1/-1;text-align:center;padding:20px;color:var(--grv-fg-dark)">No apps found</div>';
  }

  h += '</div>';
  el.innerHTML = h;

  var input = el.querySelector('.launcher-input');
  if (input) setTimeout(function() { input.focus(); input.setSelectionRange(input.value.length, input.value.length); }, 20);
}

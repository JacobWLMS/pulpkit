function renderSettings(s) {
  if (s.popup !== 'settings') return;
  const el = document.getElementById('panel-settings');
  if (!el) return;

  if (!el.dataset.built) {
    el.dataset.built = '1';
    el.innerHTML = `
      <div class="panel-title">Quick Settings</div>
      <div class="tiles">
        <div class="tile" id="tile-wifi" onclick="send({cmd:'popup',data:'wifi'})">
          <span class="tile-icon" id="tile-wifi-icon">${ICONS.wifi_4}</span>
          <span class="tile-label">WiFi</span>
        </div>
        <div class="tile" id="tile-bt" onclick="send({cmd:'toggle_bt'})">
          <span class="tile-icon">${ICONS.bt_on}</span>
          <span class="tile-label">Bluetooth</span>
        </div>
        <div class="tile" id="tile-dnd" onclick="send({cmd:'toggle_dnd'})">
          <span class="tile-icon" id="tile-dnd-icon">${ICONS.dnd_on}</span>
          <span class="tile-label">DND</span>
        </div>
        <div class="tile" id="tile-night" onclick="send({cmd:'toggle_night'})">
          <span class="tile-icon">${ICONS.night_on}</span>
          <span class="tile-label">Night</span>
        </div>
        <div class="tile" id="tile-mute" onclick="send({cmd:'vol_mute'})">
          <span class="tile-icon" id="tile-mute-icon">${ICONS.vol_hi}</span>
          <span class="tile-label">Mute</span>
        </div>
        <div class="tile" onclick="send({cmd:'screenshot'})">
          <span class="tile-icon">${ICONS.screenshot}</span>
          <span class="tile-label">Screenshot</span>
        </div>
      </div>
      <div class="sep"></div>
      <div class="slider-row">
        <span class="slider-icon" id="s-vol-icon">${ICONS.vol_hi}</span>
        <div class="slider-track">
          <div class="slider-fill" id="vol-fill"></div>
          <input type="range" id="vol-slider" min="0" max="100" value="50"
            oninput="sendThrottled('vol',{cmd:'vol_set',data:Number(this.value)})">
        </div>
        <span class="slider-val" id="s-vol-val">50%</span>
      </div>
      <div class="slider-row">
        <span class="slider-icon">${ICONS.bright}</span>
        <div class="slider-track">
          <div class="slider-fill" id="bri-fill"></div>
          <input type="range" id="bri-slider" min="1" max="100" value="50"
            oninput="sendThrottled('bri',{cmd:'bri_set',data:Number(this.value)})">
        </div>
        <span class="slider-val" id="s-bri-val">50%</span>
      </div>
      <div class="sep"></div>
      <div class="bat-row" id="bat-row">
        <span class="bat-icon" id="s-bat-icon">${ICONS.bat_full}</span>
        <span class="bat-text" id="s-bat-text">100%</span>
        <span class="bat-status" id="s-bat-status"></span>
      </div>
    `;
  }

  // Update values
  const volSlider = document.getElementById('vol-slider');
  if (volSlider) volSlider.value = s.vol;
  const volFill = document.getElementById('vol-fill');
  if (volFill) volFill.style.width = s.vol + '%';
  const sVolIcon = document.getElementById('s-vol-icon');
  if (sVolIcon) sVolIcon.textContent = volIcon(s.vol, s.muted);
  const sVolVal = document.getElementById('s-vol-val');
  if (sVolVal) sVolVal.textContent = s.vol + '%';

  const briSlider = document.getElementById('bri-slider');
  if (briSlider) briSlider.value = s.bright;
  const briFill = document.getElementById('bri-fill');
  if (briFill) briFill.style.width = s.bright + '%';
  const sBriVal = document.getElementById('s-bri-val');
  if (sBriVal) sBriVal.textContent = s.bright + '%';

  const tileMuteIcon = document.getElementById('tile-mute-icon');
  if (tileMuteIcon) tileMuteIcon.textContent = volIcon(s.vol, s.muted);
  const tileMute = document.getElementById('tile-mute');
  if (tileMute) tileMute.classList.toggle('active', s.muted);

  const tileWifiIcon = document.getElementById('tile-wifi-icon');
  if (tileWifiIcon) tileWifiIcon.textContent = s.wifi ? ICONS.wifi_4 : ICONS.wifi_off;
  const tileWifi = document.getElementById('tile-wifi');
  if (tileWifi) tileWifi.classList.toggle('active', !!s.wifi);

  const tileDnd = document.getElementById('tile-dnd');
  if (tileDnd) tileDnd.classList.toggle('active', s.dnd);
  const tileDndIcon = document.getElementById('tile-dnd-icon');
  if (tileDndIcon) tileDndIcon.textContent = s.dnd ? ICONS.dnd_on : ICONS.dnd_off;

  const batRow = document.getElementById('bat-row');
  if (s.has_bat) {
    if (batRow) batRow.style.display = '';
    const sBatText = document.getElementById('s-bat-text');
    if (sBatText) sBatText.textContent = s.bat + '%';
    const sBatStatus = document.getElementById('s-bat-status');
    if (sBatStatus) sBatStatus.textContent = s.bat_status === 'Charging' ? 'Charging' : '';
  } else {
    if (batRow) batRow.style.display = 'none';
  }
}
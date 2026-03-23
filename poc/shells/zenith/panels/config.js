function renderConfig(s) {
  if (s.popup !== 'config') return;
  const el = document.getElementById('panel-config');
  if (!el) return;

  if (!el.dataset.built) {
    el.dataset.built = '1';
    el.innerHTML = `
      <div class="panel-title">
        <span class="panel-title-icon">${ICONS.settings}</span>
        <span>Settings</span>
      </div>
      <div class="config-section">Display</div>
      <div class="slider-row">
        <span class="slider-icon">${ICONS.bright}</span>
        <div class="slider-track">
          <div class="slider-fill" id="cfg-bri-fill"></div>
          <input type="range" id="cfg-bri" min="1" max="100" value="50"
            oninput="sendThrottled('bri',{cmd:'bri_set',data:Number(this.value)})">
        </div>
        <span class="slider-val" id="cfg-bri-val">50%</span>
      </div>
      <div class="config-row">
        <span class="config-label"><span class="config-icon">${ICONS.night_on}</span> Night Light</span>
        <span class="config-value clickable" onclick="send({cmd:'toggle_night'})">Toggle</span>
      </div>
      <div class="sep"></div>
      <div class="config-section">Audio</div>
      <div class="slider-row">
        <span class="slider-icon" id="cfg-vol-icon">${ICONS.vol_hi}</span>
        <div class="slider-track">
          <div class="slider-fill" id="cfg-vol-fill"></div>
          <input type="range" id="cfg-vol" min="0" max="100" value="50"
            oninput="sendThrottled('vol',{cmd:'vol_set',data:Number(this.value)})">
        </div>
        <span class="slider-val" id="cfg-vol-val">50%</span>
      </div>
      <div class="config-row">
        <span class="config-label"><span class="config-icon">${ICONS.vol_hi}</span> Device</span>
        <span class="config-value" id="cfg-device">—</span>
      </div>
      <div class="sep"></div>
      <div class="config-section">Network</div>
      <div class="config-row">
        <span class="config-label"><span class="config-icon">${ICONS.wifi_4}</span> WiFi</span>
        <span class="config-value clickable" id="cfg-wifi" onclick="send({cmd:'popup',data:'wifi'})">—</span>
      </div>
      <div class="config-row">
        <span class="config-label"><span class="config-icon">${ICONS.ip}</span> IP</span>
        <span class="config-value" id="cfg-ip">—</span>
      </div>
      <div class="sep"></div>
      <div class="config-section">System</div>
      <div class="config-row">
        <span class="config-label"><span class="config-icon">${ICONS.mem}</span> Memory</span>
        <span class="config-value" id="cfg-mem">—</span>
      </div>
      <div class="config-row">
        <span class="config-label"><span class="config-icon">${ICONS.cpu}</span> CPU</span>
        <span class="config-value" id="cfg-cpu">—</span>
      </div>
      <div class="config-row">
        <span class="config-label"><span class="config-icon">${ICONS.disk}</span> Disk</span>
        <span class="config-value" id="cfg-disk">—</span>
      </div>
      <div class="config-row">
        <span class="config-label"><span class="config-icon">${ICONS.uptime}</span> Uptime</span>
        <span class="config-value" id="cfg-uptime">—</span>
      </div>
      <div class="config-row">
        <span class="config-label"><span class="config-icon">${ICONS.kernel}</span> Kernel</span>
        <span class="config-value" id="cfg-kernel">—</span>
      </div>
    `;
  }

  // Update values
  const cfgBri = document.getElementById('cfg-bri');
  if (cfgBri) cfgBri.value = s.bright;
  const cfgBriFill = document.getElementById('cfg-bri-fill');
  if (cfgBriFill) cfgBriFill.style.width = s.bright + '%';
  const cfgBriVal = document.getElementById('cfg-bri-val');
  if (cfgBriVal) cfgBriVal.textContent = s.bright + '%';

  const cfgVol = document.getElementById('cfg-vol');
  if (cfgVol) cfgVol.value = s.vol;
  const cfgVolFill = document.getElementById('cfg-vol-fill');
  if (cfgVolFill) cfgVolFill.style.width = s.vol + '%';
  const cfgVolVal = document.getElementById('cfg-vol-val');
  if (cfgVolVal) cfgVolVal.textContent = s.vol + '%';
  const cfgVolIcon = document.getElementById('cfg-vol-icon');
  if (cfgVolIcon) cfgVolIcon.textContent = volIcon(s.vol, s.muted);

  const cfgDevice = document.getElementById('cfg-device');
  if (cfgDevice) cfgDevice.textContent = s.audio_device || '—';
  const cfgWifi = document.getElementById('cfg-wifi');
  if (cfgWifi) cfgWifi.textContent = s.wifi || 'Disconnected';
  const cfgIp = document.getElementById('cfg-ip');
  if (cfgIp) cfgIp.textContent = s.net_ip || '—';
  const cfgMem = document.getElementById('cfg-mem');
  if (cfgMem) cfgMem.textContent = s.mem + '%';
  const cfgCpu = document.getElementById('cfg-cpu');
  if (cfgCpu) cfgCpu.textContent = s.cpu + '%';
  const cfgDisk = document.getElementById('cfg-disk');
  if (cfgDisk) cfgDisk.textContent = s.disk_used + ' / ' + s.disk_total;
  const cfgUptime = document.getElementById('cfg-uptime');
  if (cfgUptime) cfgUptime.textContent = s.uptime || '—';
  const cfgKernel = document.getElementById('cfg-kernel');
  if (cfgKernel) cfgKernel.textContent = s.kernel || '—';
}
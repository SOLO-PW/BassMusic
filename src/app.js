// BassMusic - 前端交互逻辑

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ---- 配置和常量 ----
const DEFAULT_PARAMS = {
  bass_gain_db: 6.0,
  cutoff_freq: 300.0,
  shift_ratio: 0.5,
  compress_ratio: 3.0,
  output_volume: 1.0,
};

const PARAM_PRESETS = {
  standard: {
    bass_gain_db: 6.0,
    cutoff_freq: 300.0,
    shift_ratio: 0.5,
    compress_ratio: 3.0,
    output_volume: 1.0,
  },
  strong: {
    bass_gain_db: 12.0,
    cutoff_freq: 400.0,
    shift_ratio: 0.4,
    compress_ratio: 4.0,
    output_volume: 1.1,
  },
  gentle: {
    bass_gain_db: 3.0,
    cutoff_freq: 250.0,
    shift_ratio: 0.6,
    compress_ratio: 2.0,
    output_volume: 0.9,
  },
};

const PARAM_DEBOUNCE_MS = 150;

// ---- 应用状态 ----
const state = {
  inputPath: '',
  outputPath: '',
  isConverting: false,
  isRealtimeRunning: false,
};

// ---- DOM 元素缓存 ----
const $ = (id) => document.getElementById(id);

const dom = {
  // 文件操作
  inputPath: $('input-path'),
  outputPath: $('output-path'),
  btnSelectInput: $('btn-select-input'),
  btnSelectOutput: $('btn-select-output'),
  btnCopyInput: $('btn-copy-input'),
  btnCopyOutput: $('btn-copy-output'),
  btnConvert: $('btn-convert'),
  progressFill: $('progress-fill'),
  progressText: $('progress-text'),
  
  // 实时增强
  btnStartRt: $('btn-start-rt'),
  btnStopRt: $('btn-stop-rt'),
  rtStatus: $('rt-status'),
  rtStatusText: $('rt-status-text'),
  
  // 参数控制
  btnResetParams: $('btn-reset-params'),
  sliderBassGain: $('slider-bass-gain'),
  valBassGain: $('val-bass-gain'),
  sliderCutoffFreq: $('slider-cutoff-freq'),
  valCutoffFreq: $('val-cutoff-freq'),
  sliderShiftRatio: $('slider-shift-ratio'),
  valShiftRatio: $('val-shift-ratio'),
  sliderCompressRatio: $('slider-compress-ratio'),
  valCompressRatio: $('val-compress-ratio'),
  sliderOutputVolume: $('slider-output-volume'),
  valOutputVolume: $('val-output-volume'),
  
  // 预设按钮
  presetStandard: $('preset-standard'),
  presetStrong: $('preset-strong'),
  presetGentle: $('preset-gentle'),
  
  // 其他
  statusMsg: $('status-msg'),
  btnHelp: $('btn-help'),
  btnCloseHelp: $('btn-close-help'),
  modalHelp: $('modal-help'),
};

// ---- 防抖定时器 ----
let paramDebounceTimer = null;

// ---- UI 辅助函数 ----
function setStatus(msg, type = 'info') {
  dom.statusMsg.textContent = msg;
  dom.statusMsg.className = 'status-msg';
  if (type === 'error') dom.statusMsg.classList.add('error');
  if (type === 'success') dom.statusMsg.classList.add('success');
}

function updateParamDisplay() {
  const updates = [
    [dom.valBassGain, parseFloat(dom.sliderBassGain.value).toFixed(1) + ' dB'],
    [dom.valCutoffFreq, parseFloat(dom.sliderCutoffFreq.value) + ' Hz'],
    [dom.valShiftRatio, parseFloat(dom.sliderShiftRatio.value).toFixed(2)],
    [dom.valCompressRatio, parseFloat(dom.sliderCompressRatio.value).toFixed(1)],
    [dom.valOutputVolume, parseFloat(dom.sliderOutputVolume.value) + '%']
  ];
  
  updates.forEach(([element, text]) => {
    element.textContent = text;
  });
}

function updateConvertBtnState() {
  dom.btnConvert.disabled = !(state.inputPath && state.outputPath) || state.isConverting;
}

function setProgress(percent) {
  const p = Math.min(100, Math.max(0, percent));
  dom.progressFill.style.width = p + '%';
  dom.progressText.textContent = p + '%';
}

function setBtnText(btn, text) {
  const textNode = Array.from(btn.childNodes).find(n => n.nodeType === Node.TEXT_NODE && n.textContent.trim());
  if (textNode) {
    textNode.textContent = ' ' + text;
  }
}

// ---- 核心功能函数 ----
function buildParams() {
  return {
    bass_gain_db: parseFloat(dom.sliderBassGain.value),
    cutoff_freq: parseFloat(dom.sliderCutoffFreq.value),
    shift_ratio: parseFloat(dom.sliderShiftRatio.value),
    compress_ratio: parseFloat(dom.sliderCompressRatio.value),
    output_volume: parseFloat(dom.sliderOutputVolume.value) / 100,
  };
}

async function selectInputFile() {
  try {
    const selected = await invoke('plugin:dialog|open', {
      options: {
        multiple: false, 
        directory: false, 
        title: '选择音频文件',
        filters: [
          { name: '音频文件', extensions: ['wav', 'mp3', 'flac'] }
        ]
      }
    });
    if (selected) {
      state.inputPath = selected;
      dom.inputPath.textContent = selected;
      updateConvertBtnState();
      setStatus('已选择输入文件');
    }
  } catch (e) {
    setStatus('选择文件失败: ' + e, 'error');
  }
}

async function selectOutputPath() {
  try {
    let defaultPath = 'enhanced.wav';
    if (state.inputPath) {
      const inputName = state.inputPath.split('/').pop().split('\\').pop();
      const baseName = inputName.replace(/\.[^/.]+$/, '');
      defaultPath = `${baseName}_enhanced.wav`;
    }
    
    const selected = await invoke('plugin:dialog|save', {
      options: {
        defaultPath,
        title: '保存增强音频',
        filters: [
          { name: 'WAV 音频文件', extensions: ['wav'] }
        ]
      }
    });
    if (selected) {
      state.outputPath = selected;
      dom.outputPath.textContent = selected;
      updateConvertBtnState();
      setStatus('已选择输出路径');
    }
  } catch (e) {
    setStatus('选择路径失败: ' + e, 'error');
  }
}

async function startConvert() {
  if (!state.inputPath || !state.outputPath) return;

  state.isConverting = true;
  dom.btnConvert.disabled = true;
  setBtnText(dom.btnConvert, '转化中...');
  setProgress(0);
  setStatus('正在转化...');

  try {
    const params = buildParams();
    const result = await invoke('convert_audio_file', {
      inputPath: state.inputPath,
      outputPath: state.outputPath,
      params,
    });
    setStatus('转化完成: ' + result, 'success');
  } catch (e) {
    setStatus('转化失败: ' + e, 'error');
  } finally {
    state.isConverting = false;
    setBtnText(dom.btnConvert, '开始转化');
    updateConvertBtnState();
  }
}

async function startRealtime() {
  try {
    const result = await invoke('start_realtime');
    state.isRealtimeRunning = true;
    dom.btnStartRt.disabled = true;
    dom.btnStopRt.disabled = false;
    dom.rtStatus.classList.add('active');
    dom.rtStatusText.textContent = '运行中';
    setStatus('实时增强已启动: ' + result, 'success');
  } catch (e) {
    setStatus('启动失败: ' + e, 'error');
    if (e.includes('仅在 Windows 平台上可用')) {
      dom.btnStartRt.disabled = true;
      dom.btnStartRt.title = '实时增强功能仅在 Windows 平台上可用';
    }
  }
}

async function stopRealtime() {
  try {
    const result = await invoke('stop_realtime');
    state.isRealtimeRunning = false;
    dom.btnStartRt.disabled = false;
    dom.btnStopRt.disabled = true;
    dom.rtStatus.classList.remove('active');
    dom.rtStatusText.textContent = '未运行';
    setStatus('实时增强已停止: ' + result);
  } catch (e) {
    setStatus('停止失败: ' + e, 'error');
  }
}

function onParamChange() {
  updateParamDisplay();
  if (state.isRealtimeRunning) {
    if (paramDebounceTimer) clearTimeout(paramDebounceTimer);
    paramDebounceTimer = setTimeout(async () => {
      try {
        await invoke('update_realtime_params', { params: buildParams() });
      } catch (e) {
        setStatus('更新参数失败: ' + e, 'error');
      }
    }, PARAM_DEBOUNCE_MS);
  }
}

function resetParams() {
  dom.sliderBassGain.value = DEFAULT_PARAMS.bass_gain_db;
  dom.sliderCutoffFreq.value = DEFAULT_PARAMS.cutoff_freq;
  dom.sliderShiftRatio.value = DEFAULT_PARAMS.shift_ratio;
  dom.sliderCompressRatio.value = DEFAULT_PARAMS.compress_ratio;
  dom.sliderOutputVolume.value = DEFAULT_PARAMS.output_volume * 100;
  updateParamDisplay();
  onParamChange();
  setStatus('参数已重置为默认值');
}

async function copyPath(path) {
  try {
    await navigator.clipboard.writeText(path);
    setStatus('路径已复制到剪贴板', 'success');
  } catch (e) {
    setStatus('复制路径失败: ' + e, 'error');
  }
}

function applyPreset(presetName) {
  const preset = PARAM_PRESETS[presetName];
  if (!preset) return;
  
  dom.sliderBassGain.value = preset.bass_gain_db;
  dom.sliderCutoffFreq.value = preset.cutoff_freq;
  dom.sliderShiftRatio.value = preset.shift_ratio;
  dom.sliderCompressRatio.value = preset.compress_ratio;
  dom.sliderOutputVolume.value = preset.output_volume * 100;
  
  updateParamDisplay();
  onParamChange();
  setStatus(`已应用 ${presetName === 'standard' ? '标准' : presetName === 'strong' ? '增强' : '柔和'} 预设`);
}

// ---- 事件处理 ----
function setupProgressListener() {
  listen('convert-progress', (event) => {
    const percent = Math.round(event.payload);
    setProgress(percent);
    if (percent >= 100) {
      setStatus('转化完成', 'success');
    }
  });
}

function bindEvents() {
  // 文件操作事件
  dom.btnSelectInput.addEventListener('click', selectInputFile);
  dom.btnSelectOutput.addEventListener('click', selectOutputPath);
  dom.btnCopyInput.addEventListener('click', () => copyPath(state.inputPath));
  dom.btnCopyOutput.addEventListener('click', () => copyPath(state.outputPath));
  dom.btnConvert.addEventListener('click', startConvert);
  
  // 实时增强事件
  dom.btnStartRt.addEventListener('click', startRealtime);
  dom.btnStopRt.addEventListener('click', stopRealtime);
  
  // 参数控制事件
  dom.btnResetParams.addEventListener('click', resetParams);
  
  // 预设按钮事件
  dom.presetStandard.addEventListener('click', () => applyPreset('standard'));
  dom.presetStrong.addEventListener('click', () => applyPreset('strong'));
  dom.presetGentle.addEventListener('click', () => applyPreset('gentle'));
  
  // 帮助模态框事件
  dom.btnHelp.addEventListener('click', () => {
    dom.modalHelp.hidden = false;
  });
  dom.btnCloseHelp.addEventListener('click', () => {
    dom.modalHelp.hidden = true;
  });
  dom.modalHelp.addEventListener('click', (e) => {
    if (e.target === dom.modalHelp) {
      dom.modalHelp.hidden = true;
    }
  });
  
  // 键盘事件
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && !dom.modalHelp.hidden) {
      dom.modalHelp.hidden = true;
    }
  });
  
  // 滑块事件
  const sliders = [
    dom.sliderBassGain, dom.sliderCutoffFreq, dom.sliderShiftRatio,
    dom.sliderCompressRatio, dom.sliderOutputVolume,
  ];
  sliders.forEach((slider) => {
    slider.addEventListener('input', onParamChange);
  });
}

// ---- 应用初始化 ----
async function init() {
  updateParamDisplay();
  updateConvertBtnState();
  bindEvents();
  setupProgressListener();
  setStatus('就绪');
}

document.addEventListener('DOMContentLoaded', init);

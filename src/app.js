// BassMusic - 前端交互逻辑

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

/** 默认参数值 */
const DEFAULT_PARAMS = {
  bass_gain_db: 6.0,
  cutoff_freq: 300.0,
  shift_ratio: 0.5,
  compress_ratio: 3.0,
  output_volume: 1.0,
};

/** 应用状态 */
const state = {
  inputPath: '',
  outputPath: '',
  isConverting: false,
  isRealtimeRunning: false,
};

// ---- DOM 元素缓存 ----
const $ = (id) => document.getElementById(id);

const dom = {
  inputPath: $('input-path'),
  outputPath: $('output-path'),
  btnSelectInput: $('btn-select-input'),
  btnSelectOutput: $('btn-select-output'),
  btnConvert: $('btn-convert'),
  progressFill: $('progress-fill'),
  progressText: $('progress-text'),
  btnStartRt: $('btn-start-rt'),
  btnStopRt: $('btn-stop-rt'),
  rtStatus: $('rt-status'),
  rtStatusText: $('rt-status-text'),
  btnResetParams: $('btn-reset-params'),
  statusMsg: $('status-msg'),
  btnHelp: $('btn-help'),
  btnCloseHelp: $('btn-close-help'),
  modalHelp: $('modal-help'),
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
};

/** 防抖定时器 ID */
let paramDebounceTimer = null;
/** 防抖延迟（ms） */
const PARAM_DEBOUNCE_MS = 150;

/**
 * 在状态栏显示消息，支持 info / error / success 三种类型
 */
function setStatus(msg, type = 'info') {
  dom.statusMsg.textContent = msg;
  dom.statusMsg.className = 'status-msg';
  if (type === 'error') dom.statusMsg.classList.add('error');
  if (type === 'success') dom.statusMsg.classList.add('success');
}

/**
 * 从当前滑块值构建后端所需的 params 对象
 */
function buildParams() {
  return {
    bass_gain_db: parseFloat(dom.sliderBassGain.value),
    cutoff_freq: parseFloat(dom.sliderCutoffFreq.value),
    shift_ratio: parseFloat(dom.sliderShiftRatio.value),
    compress_ratio: parseFloat(dom.sliderCompressRatio.value),
    output_volume: parseFloat(dom.sliderOutputVolume.value) / 100,
  };
}

/**
 * 更新所有滑块旁的数值显示
 */
function updateParamDisplay() {
  dom.valBassGain.textContent = parseFloat(dom.sliderBassGain.value).toFixed(1) + ' dB';
  dom.valCutoffFreq.textContent = parseFloat(dom.sliderCutoffFreq.value) + ' Hz';
  dom.valShiftRatio.textContent = parseFloat(dom.sliderShiftRatio.value).toFixed(2);
  dom.valCompressRatio.textContent = parseFloat(dom.sliderCompressRatio.value).toFixed(1);
  dom.valOutputVolume.textContent = parseFloat(dom.sliderOutputVolume.value) + '%';
}

/**
 * 根据输入/输出路径是否都已选择来启用或禁用转化按钮
 */
function updateConvertBtnState() {
  dom.btnConvert.disabled = !(state.inputPath && state.outputPath) || state.isConverting;
}

/**
 * 更新进度条 UI
 */
function setProgress(percent) {
  const p = Math.min(100, Math.max(0, percent));
  dom.progressFill.style.width = p + '%';
  dom.progressText.textContent = p + '%';
}

/**
 * 选择输入文件，通过 Tauri 文件对话框
 */
async function selectInputFile() {
  try {
    const selected = await invoke('plugin:dialog|open', { multiple: false, directory: false });
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

/**
 * 选择输出路径，通过 Tauri 保存对话框
 */
async function selectOutputPath() {
  try {
    const selected = await invoke('plugin:dialog|save', { defaultPath: 'enhanced.wav' });
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

/**
 * 更新含图标的按钮文本，保留 SVG 不被覆盖
 */
function setBtnText(btn, text) {
  const textNode = Array.from(btn.childNodes).find(n => n.nodeType === Node.TEXT_NODE && n.textContent.trim());
  if (textNode) {
    textNode.textContent = ' ' + text;
  }
}

/**
 * 执行文件转化，调用后端 convert_audio_file 命令
 */
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

/**
 * 启动实时增强，调用后端 start_realtime 命令
 */
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
  }
}

/**
 * 停止实时增强，调用后端 stop_realtime 命令
 */
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

/**
 * 当参数变化时，若实时增强运行中则防抖更新参数
 */
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

/**
 * 重置所有参数滑块到默认值
 */
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

/**
 * 打开帮助模态框
 */
function openHelp() {
  dom.modalHelp.hidden = false;
}

/**
 * 关闭帮助模态框
 */
function closeHelp() {
  dom.modalHelp.hidden = true;
}

/**
 * 监听后端 convert-progress 事件，更新进度条
 */
function setupProgressListener() {
  listen('convert-progress', (event) => {
    const percent = Math.round(event.payload);
    setProgress(percent);
    if (percent >= 100) {
      setStatus('转化完成', 'success');
    }
  });
}

/**
 * 绑定所有 DOM 事件
 */
function bindEvents() {
  dom.btnSelectInput.addEventListener('click', selectInputFile);
  dom.btnSelectOutput.addEventListener('click', selectOutputPath);
  dom.btnConvert.addEventListener('click', startConvert);
  dom.btnStartRt.addEventListener('click', startRealtime);
  dom.btnStopRt.addEventListener('click', stopRealtime);
  dom.btnResetParams.addEventListener('click', resetParams);
  dom.btnHelp.addEventListener('click', openHelp);
  dom.btnCloseHelp.addEventListener('click', closeHelp);

  dom.modalHelp.addEventListener('click', (e) => {
    if (e.target === dom.modalHelp) closeHelp();
  });

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && !dom.modalHelp.hidden) closeHelp();
  });

  const sliders = [
    dom.sliderBassGain, dom.sliderCutoffFreq, dom.sliderShiftRatio,
    dom.sliderCompressRatio, dom.sliderOutputVolume,
  ];
  sliders.forEach((slider) => {
    slider.addEventListener('input', onParamChange);
  });
}

/**
 * 应用初始化，在 DOM 加载完成后执行
 */
async function init() {
  updateParamDisplay();
  updateConvertBtnState();
  bindEvents();
  setupProgressListener();
  setStatus('就绪');
}

document.addEventListener('DOMContentLoaded', init);

// BassMusic - 前端交互逻辑

/** Tauri API 快捷引用 */
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

/** 参数预设方案 */
const PRESETS = {
  default: {
    name: '默认设置',
    params: {
      bass_gain_db: 6.0,
      cutoff_freq: 300.0,
      shift_ratio: 0.5,
      compress_ratio: 3.0,
      output_volume: 1.0,
    }
  },
  light: {
    name: '轻度增强',
    params: {
      bass_gain_db: 4.0,
      cutoff_freq: 250.0,
      shift_ratio: 0.4,
      compress_ratio: 2.0,
      output_volume: 1.0,
    }
  },
  deep: {
    name: '深度增强',
    params: {
      bass_gain_db: 12.0,
      cutoff_freq: 350.0,
      shift_ratio: 0.6,
      compress_ratio: 4.0,
      output_volume: 1.1,
    }
  }
};

/** 应用状态 */
const state = {
  inputPath: '',
  outputPath: '',
  isConverting: false,
  isRealtimeRunning: false,
};

/** 最近使用文件的最大数量 */
const MAX_RECENT_FILES = 5;

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
  // 滑块和值显示
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
  // 预设选择
  presetSelect: $('preset-select'),
};

/**
 * 在状态栏显示消息，支持 info / error / success 三种类型
 */
function setStatus(msg, type = 'info', duration = 3000) {
  dom.statusMsg.textContent = msg;
  dom.statusMsg.className = 'status-msg';
  if (type === 'error') dom.statusMsg.classList.add('error');
  if (type === 'success') dom.statusMsg.classList.add('success');
  
  // 清除之前的定时器
  if (window.statusTimeout) {
    clearTimeout(window.statusTimeout);
  }
  
  // 设置自动消失
  if (duration > 0) {
    window.statusTimeout = setTimeout(() => {
      dom.statusMsg.textContent = '就绪';
      dom.statusMsg.className = 'status-msg';
    }, duration);
  }
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
      addToRecentFiles(selected);
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
  if (!state.inputPath) {
    setStatus('请先选择输入文件', 'error');
    return;
  }
  if (!state.outputPath) {
    setStatus('请先选择输出路径', 'error');
    return;
  }

  state.isConverting = true;
  dom.btnConvert.disabled = true;
  setBtnText(dom.btnConvert, '转化中...');
  setProgress(0);
  setStatus('正在转化...', 'info', 0);

  try {
    const params = buildParams();
    const result = await invoke('convert_audio_file', {
      inputPath: state.inputPath,
      outputPath: state.outputPath,
      params,
    });
    setStatus('转化完成: ' + result, 'success');
  } catch (e) {
    let errorMsg = '转化失败';
    if (e.message) {
      errorMsg += ': ' + e.message;
    } else if (e) {
      errorMsg += ': ' + e;
    }
    setStatus(errorMsg, 'error');
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
 * 当参数变化时，若实时增强运行中则自动更新参数
 */
async function onParamChange() {
  updateParamDisplay();
  saveParamsToStorage();
  if (state.isRealtimeRunning) {
    try {
      await invoke('update_realtime_params', { params: buildParams() });
    } catch (e) {
      setStatus('更新参数失败: ' + e, 'error');
    }
  }
}

/**
 * 应用预设参数
 */
function applyPreset(presetKey) {
  if (PRESETS[presetKey]) {
    const preset = PRESETS[presetKey];
    const params = preset.params;
    
    dom.sliderBassGain.value = params.bass_gain_db;
    dom.sliderCutoffFreq.value = params.cutoff_freq;
    dom.sliderShiftRatio.value = params.shift_ratio;
    dom.sliderCompressRatio.value = params.compress_ratio;
    dom.sliderOutputVolume.value = params.output_volume * 100;
    
    updateParamDisplay();
    saveParamsToStorage();
    onParamChange();
    setStatus(`已应用预设: ${preset.name}`);
  }
}

/**
 * 重置所有参数滑块到默认值
 */
function resetParams() {
  applyPreset('default');
  dom.presetSelect.value = 'default';
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

  // 点击模态框背景关闭
  dom.modalHelp.addEventListener('click', (e) => {
    if (e.target === dom.modalHelp) closeHelp();
  });

  // ESC 键关闭模态框
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && !dom.modalHelp.hidden) closeHelp();
  });

  // 快捷键支持
  document.addEventListener('keydown', (e) => {
    // Ctrl+O: 选择输入文件
    if (e.ctrlKey && e.key === 'o') {
      e.preventDefault();
      selectInputFile();
    }
    // Ctrl+S: 选择输出路径
    else if (e.ctrlKey && e.key === 's') {
      e.preventDefault();
      selectOutputPath();
    }
    // Ctrl+Enter: 开始转化
    else if (e.ctrlKey && e.key === 'Enter') {
      e.preventDefault();
      if (!state.isConverting && state.inputPath && state.outputPath) {
        startConvert();
      }
    }
    // F1: 打开帮助
    else if (e.key === 'F1') {
      e.preventDefault();
      openHelp();
    }
  });

  // 预设选择事件
  dom.presetSelect.addEventListener('change', (e) => {
    const presetKey = e.target.value;
    if (presetKey !== 'custom') {
      applyPreset(presetKey);
    }
  });

  // 参数滑块变化事件
  const sliders = [
    dom.sliderBassGain, dom.sliderCutoffFreq, dom.sliderShiftRatio,
    dom.sliderCompressRatio, dom.sliderOutputVolume,
  ];
  sliders.forEach((slider) => {
    slider.addEventListener('input', () => {
      onParamChange();
      // 当参数变化时，将预设设置为自定义
      dom.presetSelect.value = 'custom';
    });
  });
}

/**
 * 处理文件拖拽事件
 */
function setupDragAndDrop() {
  const app = document.getElementById('app');
  
  // 阻止默认拖拽行为
  app.addEventListener('dragover', (e) => {
    e.preventDefault();
    e.stopPropagation();
  });
  
  app.addEventListener('dragenter', (e) => {
    e.preventDefault();
    e.stopPropagation();
  });
  
  app.addEventListener('dragleave', (e) => {
    e.preventDefault();
    e.stopPropagation();
  });
  
  // 处理文件放置
  app.addEventListener('drop', (e) => {
    e.preventDefault();
    e.stopPropagation();
    
    if (e.dataTransfer.files.length > 0) {
      const file = e.dataTransfer.files[0];
      // 检查是否为音频文件
      if (file.type.startsWith('audio/')) {
        state.inputPath = file.path;
        dom.inputPath.textContent = file.path;
        addToRecentFiles(file.path);
        updateConvertBtnState();
        setStatus('已通过拖拽选择输入文件');
      } else {
        setStatus('请拖拽音频文件', 'error');
      }
    }
  });
}

/**
 * 从本地存储加载参数设置
 */
function loadParamsFromStorage() {
  try {
    const savedParams = localStorage.getItem('bassmusic_params');
    if (savedParams) {
      const params = JSON.parse(savedParams);
      dom.sliderBassGain.value = params.bass_gain_db || DEFAULT_PARAMS.bass_gain_db;
      dom.sliderCutoffFreq.value = params.cutoff_freq || DEFAULT_PARAMS.cutoff_freq;
      dom.sliderShiftRatio.value = params.shift_ratio || DEFAULT_PARAMS.shift_ratio;
      dom.sliderCompressRatio.value = params.compress_ratio || DEFAULT_PARAMS.compress_ratio;
      dom.sliderOutputVolume.value = (params.output_volume || DEFAULT_PARAMS.output_volume) * 100;
      updateParamDisplay();
      setStatus('已加载保存的参数设置');
    }
  } catch (e) {
    console.error('加载参数失败:', e);
  }
}

/**
 * 保存参数设置到本地存储
 */
function saveParamsToStorage() {
  try {
    const params = buildParams();
    localStorage.setItem('bassmusic_params', JSON.stringify(params));
  } catch (e) {
    console.error('保存参数失败:', e);
  }
}

/**
 * 保存文件到最近使用列表
 */
function addToRecentFiles(filePath) {
  try {
    let recentFiles = JSON.parse(localStorage.getItem('bassmusic_recent_files') || '[]');
    
    // 移除已存在的相同文件
    recentFiles = recentFiles.filter(file => file !== filePath);
    
    // 添加到列表开头
    recentFiles.unshift(filePath);
    
    // 限制列表长度
    if (recentFiles.length > MAX_RECENT_FILES) {
      recentFiles = recentFiles.slice(0, MAX_RECENT_FILES);
    }
    
    localStorage.setItem('bassmusic_recent_files', JSON.stringify(recentFiles));
  } catch (e) {
    console.error('保存最近文件失败:', e);
  }
}

/**
 * 加载最近使用文件列表
 */
function getRecentFiles() {
  try {
    return JSON.parse(localStorage.getItem('bassmusic_recent_files') || '[]');
  } catch (e) {
    console.error('加载最近文件失败:', e);
    return [];
  }
}

/**
 * 显示最近使用文件列表
 */
function showRecentFiles() {
  const recentFiles = getRecentFiles();
  if (recentFiles.length === 0) return;
  
  // 这里可以实现一个下拉菜单来显示最近使用的文件
  // 为了简单起见，我们先在控制台打印
  console.log('最近使用的文件:', recentFiles);
}

/**
 * 应用初始化，在 DOM 加载完成后执行
 */
async function init() {
  loadParamsFromStorage();
  updateParamDisplay();
  updateConvertBtnState();
  bindEvents();
  setupProgressListener();
  setupDragAndDrop();
  setStatus('就绪');
}

document.addEventListener('DOMContentLoaded', init);

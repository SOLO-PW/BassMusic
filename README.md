# BassMusic

**听障人士音频增强工具** — 通过低频增强、高频下移和动态压缩，帮助听障人士更清晰地感知音频内容。

---

## 目录

- [功能概览](#功能概览)
- [工作原理](#工作原理)
- [技术栈](#技术栈)
- [环境要求](#环境要求)
- [快速开始](#快速开始)
- [使用指南](#使用指南)
- [API 参考](#api-参考)
- [项目结构](#项目结构)
- [开发指南](#开发指南)
- [故障排除](#故障排除)
- [贡献指南](#贡献指南)
- [许可证](#许可证)

---

## 功能概览

### 文件转化模式

将本地音频文件经 DSP 管线处理后输出增强版本。

| 特性 | 说明 |
|------|------|
| 输入格式 | WAV、MP3、FLAC |
| 输出格式 | 16-bit PCM WAV（单声道） |
| 进度反馈 | 通过 Tauri Event 实时推送转化进度 |
| 文件命名 | 自动根据输入文件名生成 `*_enhanced.wav` |

### 实时增强模式

捕获系统音频输出，经 DSP 增强后实时播放。

| 特性 | 说明 |
|------|------|
| 捕获方式 | WASAPI Loopback（捕获系统音频输出） |
| 参数热更新 | 运行中调整参数，150ms 防抖后自动生效 |
| 平台限制 | **仅 Windows**（依赖 WASAPI 音频接口） |
| 防反馈 | 建议使用耳机播放，避免扬声器反馈回路 |

### 音频处理参数

| 参数 | 范围 | 默认值 | 说明 |
|------|------|--------|------|
| 低频增益 (bass_gain_db) | 0 ~ 20 dB | 6.0 | 增强截止频率以下的低频音量 |
| 截止频率 (cutoff_freq) | 100 ~ 500 Hz | 300.0 | 低频增强的频率上限 |
| 频移比率 (shift_ratio) | 0.1 ~ 1.0 | 0.5 | 高频信号下移比率，0.5 表示频率 f 移至 f×0.5 |
| 压缩比 (compress_ratio) | 1 ~ 10 | 3.0 | 动态范围压缩比（3:1） |
| 压缩阈值 (threshold_db) | — | -20.0 | 压缩器启动阈值（dB） |
| 启动时间 (attack_ms) | — | 10.0 | 压缩器响应时间（ms） |
| 释放时间 (release_ms) | — | 100.0 | 压缩器恢复时间（ms） |
| 输出音量 (output_volume) | 0.5 ~ 1.5 | 1.0 | 最终输出音量倍率（1.0 = 100%） |

> **注**：`threshold_db`、`attack_ms`、`release_ms` 为高级参数，前端界面暂未暴露滑块控件，但可通过 API 设置。

### 预设配置

| 预设 | 低频增益 | 截止频率 | 频移比率 | 压缩比 | 输出音量 | 适用场景 |
|------|----------|----------|----------|--------|----------|----------|
| 标准 | 6.0 dB | 300 Hz | 0.50 | 3.0 | 100% | 日常使用，平衡的默认配置 |
| 增强 | 12.0 dB | 400 Hz | 0.40 | 4.0 | 110% | 需要更强低频效果的场景 |
| 柔和 | 3.0 dB | 250 Hz | 0.60 | 2.0 | 90% | 长时间使用，温和增强 |

---

## 工作原理

### DSP 处理管线

音频信号按以下顺序经过三级处理：

```
输入信号 → BassBoost → FrequencyShift → Compressor → 音量缩放 → 输出信号
```

1. **BassBoost（低频增强）**：基于 FFT 的频域滤波器，对截止频率以下的频段施加指定增益（dB），使用 Hann 窗减少频谱泄漏
2. **FrequencyShift（高频下移）**：基于 FFT 的频域下移，将高频段信号搬移到低频区域，帮助高频听力损失用户感知高频信息
3. **Compressor（动态压缩）**：Envelope Follower + 增益计算，压缩动态范围使弱信号更易感知，避免强信号刺耳

### 实时增强架构

```
系统音频输出 → WASAPI Loopback 捕获 → crossbeam 有界缓冲区 → DSP 管线 → 音频输出播放
```

- 使用 `cpal` 库访问 WASAPI Loopback 接口捕获系统音频
- crossbeam 有界通道作为音频缓冲区（容量约 2 秒），平衡延迟与抗抖动
- 输入流与输出流在同一设备上运行，通过 `AtomicBool` + `Mutex` 实现跨线程状态同步

---

## 技术栈

| 层级 | 技术 | 用途 |
|------|------|------|
| 前端 | HTML5 / CSS3 / JavaScript | 界面结构与交互 |
| 框架 | Tauri v2 | Windows 桌面应用 |
| 后端 | Rust | 核心音频处理逻辑 |
| 音频解码 | symphonia | WAV/MP3/FLAC 解码 |
| 音频编码 | hound | WAV 编码 |
| 音频 I/O | cpal | 实时音频捕获与播放 |
| DSP | rustfft | FFT 变换 |
| 并发 | crossbeam-channel | 实时音频缓冲区 |

---

## 环境要求

### 操作系统

| 平台 | 文件转化 | 实时增强 |
|------|----------|----------|
| Windows 10/11 | ✅ | ✅ |

> 目前仅支持 Windows 平台，macOS / Linux 支持计划中。

### 开发依赖

| 依赖 | 最低版本 | 安装方式 |
|------|----------|----------|
| Node.js | v16+ | [nodejs.org](https://nodejs.org) |
| Rust | v1.60+ | [rustup.rs](https://rustup.rs) |
| Tauri CLI | v2 | 随 `npm install` 自动安装 |
| C++ 构建工具 | — | Windows: Visual Studio Build Tools |

> **Windows 用户**：需安装 [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)，确保勾选 "C++ 桌面开发" 工作负载。Tauri v2 的系统依赖详见 [Tauri 官方文档](https://v2.tauri.app/start/prerequisites/)。

---

## 快速开始

```bash
# 1. 克隆项目
git clone https://github.com/yourusername/bass-music.git
cd bass-music

# 2. 安装前端依赖
npm install

# 3. 开发模式运行（热重载）
npm run dev

# 4. 构建发布版本
npm run build
```

构建产物位于 `src-tauri/target/release/` 目录。

---

## 使用指南

### 文件转化

1. 点击 **「选择文件」**，选择需要增强的音频文件（WAV / MP3 / FLAC）
2. 点击 **「选择路径」**，设置输出文件保存位置（默认生成 `*_enhanced.wav`）
3. 调整音频处理参数，或选择预设配置
4. 点击 **「开始转化」**，等待进度条完成
5. 转化完成后，状态栏显示成功信息

### 实时增强

1. 点击 **「启动」**，开始实时捕获并增强系统音频
2. 运行中调整参数，效果会在 150ms 内自动生效
3. 点击 **「停止」**，结束实时增强

> ⚠️ **重要**：实时增强会捕获并播放系统音频。如果使用扬声器播放，会产生反馈回路（扬声器输出 → 再次被捕获 → 循环放大）。**请务必使用耳机**。

### 参数说明

- **低频增益**：增大低频声音的音量。值越大，低频越强，但过高可能导致失真
- **截止频率**：低频增强作用的频率上限。300 Hz 以下为人声基频和低音乐器的主要频段
- **频移比率**：将高频信号下移到可感知的低频区域。值越小，下移幅度越大
- **压缩比**：压缩动态范围，使安静部分更响亮、响亮部分不过载。适合听力动态范围缩小的用户
- **输出音量**：最终输出的整体音量倍率

---

## API 参考

所有 API 通过 Tauri 的 `invoke` 机制从前端调用。

### `convert_audio_file`

转化音频文件，异步执行，通过事件推送进度。

```javascript
const result = await invoke('convert_audio_file', {
  inputPath: 'C:/music/song.mp3',
  outputPath: 'C:/music/song_enhanced.wav',
  params: {
    bass_gain_db: 6.0,
    cutoff_freq: 300.0,
    shift_ratio: 0.5,
    compress_ratio: 3.0,
    threshold_db: -20.0,    // 可选，默认 -20.0
    attack_ms: 10.0,        // 可选，默认 10.0
    release_ms: 100.0,      // 可选，默认 100.0
    output_volume: 1.0,
  },
});
// 返回: "转化完成！输出文件: C:/music/song_enhanced.wav"
// 错误: "已有文件正在转化中" / "不支持的音频格式" / 解码/编码错误
```

### `start_realtime`

启动实时音频增强。

```javascript
const result = await invoke('start_realtime');
// 返回: "实时增强已启动"
// 错误: "实时增强已在运行中" / "无法获取音频输出设备" / "实时增强功能仅在 Windows 平台上可用"
```

### `stop_realtime`

停止实时音频增强。

```javascript
const result = await invoke('stop_realtime');
// 返回: "实时增强已停止"
// 错误: "实时增强未在运行"
```

### `update_realtime_params`

热更新实时增强参数，无需重启音频流。

```javascript
await invoke('update_realtime_params', {
  params: {
    bass_gain_db: 8.0,
    cutoff_freq: 350.0,
    shift_ratio: 0.4,
    compress_ratio: 3.5,
    threshold_db: -20.0,
    attack_ms: 10.0,
    release_ms: 100.0,
    output_volume: 1.0,
  },
});
// 返回: "参数已更新"
```

### 事件

| 事件名 | Payload | 说明 |
|--------|---------|------|
| `convert-progress` | `u32` (0~100) | 文件转化进度百分比 |

```javascript
const { listen } = window.__TAURI__.event;
const unlisten = await listen('convert-progress', (event) => {
  const percent = Math.round(event.payload);
  console.log(`转化进度: ${percent}%`);
});
```

### EnhanceParams 完整字段

| 字段 | 类型 | 范围 | 默认值 | 说明 |
|------|------|------|--------|------|
| `bass_gain_db` | f64 | 0 ~ 20 | 6.0 | 低频增益（dB） |
| `cutoff_freq` | f64 | 100 ~ 500 | 300.0 | 截止频率（Hz） |
| `shift_ratio` | f64 | 0.1 ~ 1.0 | 0.5 | 频移比率 |
| `compress_ratio` | f64 | 1 ~ 10 | 3.0 | 压缩比 |
| `threshold_db` | f64 | — | -20.0 | 压缩阈值（dB） |
| `attack_ms` | f64 | — | 10.0 | 启动时间（ms） |
| `release_ms` | f64 | — | 100.0 | 释放时间（ms） |
| `output_volume` | f64 | 0.5 ~ 1.5 | 1.0 | 输出音量倍率 |

---

## 项目结构

```
bass-music/
├── src/                          # 前端代码
│   ├── index.html                # 主页面
│   ├── app.js                    # 交互逻辑（状态管理、API 调用、事件绑定）
│   └── styles.css                # 样式文件
├── src-tauri/                    # Tauri 后端代码
│   ├── src/
│   │   ├── main.rs               # 应用入口
│   │   ├── lib.rs                # 核心入口（注册 Commands、AppState）
│   │   ├── audio.rs              # 音频文件解码/编码（symphonia + hound）
│   │   ├── realtime.rs           # 实时音频捕获与播放（cpal + WASAPI）
│   │   └── dsp/                  # 数字信号处理模块
│   │       ├── mod.rs            # 模块导出与单元测试
│   │       ├── pipeline.rs       # 处理管线编排 + EnhanceParams 定义
│   │       ├── bass_boost.rs     # 低频增强（FFT 频域滤波）
│   │       ├── frequency_shift.rs # 高频下移（FFT 频域搬移）
│   │       └── compressor.rs     # 动态压缩（Envelope Follower）
│   ├── capabilities/
│   │   └── default.json          # Tauri 权限配置
│   ├── icons/                    # 应用图标
│   ├── Cargo.toml                # Rust 依赖配置
│   ├── build.rs                  # Tauri 构建脚本
│   └── tauri.conf.json           # Tauri 应用配置
├── package.json                  # NPM 配置
├── LICENSE                       # 许可证
└── README.md                     # 本文档
```

---

## 开发指南

### 常用命令

```bash
# 开发模式（前端热重载 + Rust 增量编译）
npm run dev

# 构建发布版本
npm run build

# 运行 Rust 单元测试
cd src-tauri && cargo test

# 检查 Rust 代码
cd src-tauri && cargo clippy
```

### 架构要点

- **前后端通信**：前端通过 `window.__TAURI__.core.invoke()` 调用后端 `#[tauri::command]` 函数
- **状态管理**：`AppState` 通过 `tauri::State` 注入，包含 `RealtimeState`（实时增强状态）和 `AtomicBool`（转化锁）
- **实时参数更新**：前端滑块变更经 150ms 防抖后调用 `update_realtime_params`，后端通过 `Arc<Mutex<EnhanceParams>>` 实现无锁读取
- **文件转化并发控制**：`AtomicBool` 防止重复触发转化，`spawn_blocking` 避免阻塞 Tauri 主线程
- **实时音频流安全**：`cpal::Stream` 本身 `!Send`，通过 `SendableStream` 包装（仅 WASAPI MTA 模式下安全）

---

## 故障排除

### 常见问题

**Q: 实时增强启动失败，提示"无法获取音频输出设备"**

确保系统已连接音频输出设备（扬声器或耳机），且未被其他应用独占。在 Windows 声音设置中检查默认输出设备。

**Q: 实时增强有明显的回声或啸叫**

这是反馈回路导致的——扬声器播放的增强音频再次被 Loopback 捕获。请使用耳机播放。

**Q: 文件转化提示"不支持的音频格式"**

确认输入文件扩展名为 `.wav`、`.mp3` 或 `.flac`。其他格式（如 `.aac`、`.ogg`）暂不支持。

**Q: 转化后的音频音量很小或失真**

- 检查 `output_volume` 参数是否设置过低
- 降低 `bass_gain_db` 避免低频削波
- 增大 `compress_ratio` 压缩动态范围

**Q: macOS / Linux 上可以使用吗？**

目前仅支持 Windows 平台（依赖 WASAPI 音频接口），macOS / Linux 支持计划中。

---

## 贡献指南

欢迎贡献代码、报告问题或完善文档。

### 流程

1. Fork 本仓库
2. 创建功能分支：`git checkout -b feature/your-feature`
3. 提交更改：`git commit -m 'Add your feature'`
4. 推送分支：`git push origin feature/your-feature`
5. 创建 Pull Request

### 规范

- **Rust 代码**：遵循 `rustfmt` + `clippy` 规范
- **JavaScript 代码**：保持与现有风格一致
- **提交信息**：简洁明确，描述做了什么
- **测试**：新增 DSP 功能需补充单元测试（参考 `dsp/mod.rs` 中的测试用例）

---

## 许可证

本项目采用 [GNU General Public License v3.0](LICENSE) 许可证。

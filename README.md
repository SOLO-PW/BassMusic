# BassMusic - 听障人士音频增强工具

## 项目概述

BassMusic 是一款专为听障人士设计的音频增强工具，通过增强低频声音和优化音频特性，帮助听障人士更清晰地感知音频内容。该应用支持两种工作模式：文件转化模式和实时增强模式，可根据不同场景提供定制化的音频增强解决方案。

![BassMusic 界面](file:///workspace/app-icon.png)

## 核心功能与特性

### 1. 文件转化模式
- 支持 WAV、MP3、FLAC 等常见音频格式
- 处理后保存为高质量 WAV 文件
- 实时显示转化进度
- 智能生成默认输出文件名

### 2. 实时增强模式
- 实时捕获并增强系统音频输出
- 参数调整实时生效
- 仅在 Windows 平台上可用（依赖系统音频捕获功能）

### 3. 专业音频处理参数
- **低频增益**：0-20 dB，增强低频音量
- **截止频率**：100-500 Hz，设置低频增强的频率上限
- **频移比率**：0.1-1.0，将高频信号下移到低频区域
- **压缩比**：1-10，优化动态范围
- **输出音量**：50%-150%，调整最终输出音量

### 4. 预设配置
- **标准**：平衡的默认配置
- **增强**：更强的低频增强效果
- **柔和**：温和的增强效果，适合长时间使用

### 5. 其他特性
- 响应式界面设计
- 实时状态反馈
- 路径复制功能
- 详细的帮助文档

## 技术栈选型

### 前端
- **HTML5**：页面结构
- **CSS3**：样式设计
- **JavaScript**：交互逻辑

### 后端
- **Rust**：核心音频处理逻辑
- **Tauri**：跨平台桌面应用框架
  - @tauri-apps/api：Tauri API 调用
  - @tauri-apps/plugin-dialog：文件选择对话框

### 音频处理
- **Rust 音频处理库**：实现 DSP（数字信号处理）功能
  - 低频增强（Bass Boost）
  - 频率转换（Frequency Shift）
  - 压缩器（Compressor）
  - 音频处理管道（Pipeline）

## 环境配置与安装步骤

### 系统要求
- **Windows**：支持实时增强功能
- **macOS**：仅支持文件转化功能
- **Linux**：仅支持文件转化功能

### 依赖项
- Node.js (v16+)
- Rust (v1.60+)
- Tauri CLI

### 安装步骤

1. **克隆项目**
   ```bash
   git clone https://github.com/yourusername/bass-music.git
   cd bass-music
   ```

2. **安装依赖**
   ```bash
   npm install
   ```

3. **开发模式运行**
   ```bash
   npm run dev
   ```

4. **构建应用**
   ```bash
   npm run build
   ```
   构建产物将位于 `src-tauri/target/release` 目录中

## 使用指南

### 文件转化模式
1. 点击「选择文件」按钮，选择需要增强的音频文件
2. 点击「选择路径」按钮，设置增强后文件的保存位置
3. 调整音频处理参数（或使用预设）
4. 点击「开始转化」按钮，等待处理完成
5. 处理完成后，状态栏会显示成功信息

### 实时增强模式
1. 点击「启动」按钮，开始实时音频增强
2. 调整音频处理参数，效果会实时生效
3. 点击「停止」按钮，结束实时增强

### 参数调节
- **低频增益**：增加低频声音的音量
- **截止频率**：设置需要增强的低频范围上限
- **频移比率**：控制高频信号向低频区域的转移程度
- **压缩比**：减少音频动态范围，使音量更加均匀
- **输出音量**：调整最终输出的整体音量

### 预设使用
- **标准**：适合大多数场景的默认配置
- **增强**：适合需要更强低频效果的场景
- **柔和**：适合长时间使用，效果较为温和

## API 接口说明

### 前端调用后端 API

1. **文件转化**
   ```javascript
   const result = await invoke('convert_audio_file', {
     inputPath: 'path/to/input/file',
     outputPath: 'path/to/output/file',
     params: {
       bass_gain_db: 6.0,
       cutoff_freq: 300.0,
       shift_ratio: 0.5,
       compress_ratio: 3.0,
       output_volume: 1.0
     }
   });
   ```

2. **启动实时增强**
   ```javascript
   const result = await invoke('start_realtime');
   ```

3. **停止实时增强**
   ```javascript
   const result = await invoke('stop_realtime');
   ```

4. **更新实时增强参数**
   ```javascript
   await invoke('update_realtime_params', {
     params: {
       bass_gain_db: 8.0,
       cutoff_freq: 350.0,
       shift_ratio: 0.4,
       compress_ratio: 3.5,
       output_volume: 1.0
     }
   });
   ```

### 事件监听

1. **转化进度事件**
   ```javascript
   listen('convert-progress', (event) => {
     const percent = Math.round(event.payload);
     // 更新进度条
   });
   ```

## 项目结构说明

```
├── src/                 # 前端代码
│   ├── app.js           # 前端交互逻辑
│   ├── index.html       # 主页面
│   └── styles.css       # 样式文件
├── src-tauri/           # Tauri 后端代码
│   ├── capabilities/    # 权限配置
│   ├── gen/             # 生成的代码
│   ├── icons/           # 应用图标
│   ├── src/             # Rust 源代码
│   │   ├── dsp/         # 数字信号处理模块
│   │   │   ├── bass_boost.rs     # 低频增强
│   │   │   ├── compressor.rs     # 压缩器
│   │   │   ├── frequency_shift.rs # 频率转换
│   │   │   ├── mod.rs            # 模块导出
│   │   │   └── pipeline.rs       # 音频处理管道
│   │   ├── audio.rs     # 音频处理核心
│   │   ├── lib.rs       # 库入口
│   │   ├── main.rs      # 应用入口
│   │   └── realtime.rs  # 实时处理模块
│   ├── Cargo.toml       # Rust 依赖配置
│   ├── build.rs         # 构建脚本
│   └── tauri.conf.json  # Tauri 配置
├── .gitignore           # Git 忽略文件
├── LICENSE              # 许可证文件
├── app-icon.png         # 应用图标
├── package-lock.json    # NPM 依赖锁定
└── package.json         # NPM 配置
```

## 贡献指南

我们欢迎社区贡献，无论是功能改进、Bug 修复还是文档完善。

### 贡献流程
1. Fork 项目仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 打开 Pull Request

### 开发规范
- 代码风格：遵循 Rust 官方风格指南和 JavaScript 标准风格
- 提交信息：使用清晰、简洁的提交信息
- 测试：确保新功能有适当的测试覆盖
- 文档：更新相关文档以反映更改

## 许可证信息

本项目采用 MIT 许可证。详见 [LICENSE](file:///workspace/LICENSE) 文件。

## 联系方式

如有问题或建议，欢迎通过以下方式联系我们：

- 项目地址：[https://github.com/yourusername/bass-music](https://github.com/yourusername/bass-music)
- 电子邮件：contact@bassmusic.app

---

**BassMusic** - 让每个人都能享受清晰的音频体验

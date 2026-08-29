# 🗺️ PhotoMap 智能照片地图

> 把一整个文件夹的照片，变成一张会讲故事的旅行地图。


<p align="center">
  <a href="https://github.com/cuoduidui/photomap"><img src="https://img.shields.io/badge/platform-Windows-0078d6.svg" alt="Platform: Windows"/></a>
  <a href="https://github.com/cuoduidui/photomap"><img src="https://img.shields.io/badge/i18n-7%20languages-green.svg" alt="7 Languages"/></a>
  <a href="https://github.com/cuoduidui/photomap"><img src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg" alt="PRs Welcome"/></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"/></a>
</p>

PhotoMap 是一款本地优先的 Windows 桌面应用：导入照片后自动按 GPS 定位到地图，按时间轴回放旅程，并用 AI 一键生成图文游记。所有照片与数据只保存在你自己的设备上。

> 项目仓库：[https://github.com/cuoduidui/photomap](https://github.com/cuoduidui/photomap)

## ✨ 功能特性

### 🗺️ 照片地图
- 带 GPS 的照片自动落点，支持聚合标记与缩放
- 地图与筛选实时联动：地点、时间过滤后，标记数量与照片列表完全一致
- 路线回放：按拍摄时间把照片连成线，在地图上重走旅程
- 点击地点节点，地图自动移动并分级缩放

### 📍 地点分类树
- 「省 → 市 → 区县 → 街道 → 地址」多级折叠，默认收起、点击展开
- 中文地址自动识别（高德逆地理编码），纯坐标照片单独分组
- 点击节点 = 筛选照片 + 地图定位

### ⏳ 时间轴
- 按天显示照片密度，拖动区间过滤地图标记与照片列表
- 一眼看清「某一天我去了哪里」

### ✍️ AI 游记
- 一键生成 800–1200 字图文游记，自动识别地点、人物、拍摄设备
- 手动编辑 + AI 润色，游记中提到的照片可点击定位
- 输出语言跟随界面语言（7 种）

### 🖼️ 相册与影集
- 主题相册、照片拼图导出
- 带背景音乐与旁白的 MP4 影集视频（需 ffmpeg）

### 🏷️ 标签与人脸
- 自定义标签 + 人脸分析，按人物快速找照片

### 🌈 界面与主题
- 7 种界面语言：简体中文 / English / 日本語 / Français / 한국어 / Deutsch / Русский
- 多套配色主题一键切换
- API Key 使用 Windows DPAPI 加密存储

## 📸 截图

<!-- 截图占位：图片放入 docs/screenshots/ 后取消注释 -->
<!-- ![主界面](docs/screenshots/main.png) -->

## 🛠️ 技术栈

| 层 | 技术 |
|---|---|
| 前端 | Vue 3 · Vite · Pinia · vue-i18n |
| 桌面框架 | Tauri 2 (Rust) · WebView2 |
| 数据库 | SQLite（rusqlite） |
| 地图 | 高德地图 JS API + Web 服务 API |
| 图片处理 | 自研 EXIF 解析 + 缩略图引擎 |
| 视频合成 | ffmpeg |

## 📁 项目结构

```
photomap/
├── src/                  # 前端（Vue 3）
│   ├── components/       # 地图 / 照片 / 游记 / 设置等组件
│   ├── i18n/             # 7 语言语言包与后端错误翻译
│   ├── stores/           # Pinia 状态
│   ├── utils/            # Tauri 调用 / 坐标转换 / 主题
│   └── App.vue
├── src-tauri/            # Rust 后端
│   └── src/              # 命令、数据库、地理编码、影集合成
├── docs/                 # 设计文档
├── scripts/              # 构建辅助脚本
├── store/                # 应用商店素材（可忽略）
└── package.json
```

## ✅ 环境要求

- Windows 10/11（需 WebView2 运行时）
- Node.js 18+ 与 npm
- Rust stable（MSVC 工具链）
- ffmpeg（可选，仅影集视频需要）
- 高德开放平台账号（地图显示 + 逆地理编码）

## 🚀 快速开始

```bash
npm install
npm run tauri dev      # 开发模式运行
npm run tauri build    # 构建安装包（NSIS / MSI）
```

## 📖 使用指南

### 1. 配置 API Key（首次使用必做）

打开「设置」：

- **高德地图 API Key**：到 [高德开放平台](https://lbs.amap.com/) 申请。地图显示需要「Web端(JS API)」类型，逆地理编码需要「Web服务」类型（可在控制台为同一个 Key 勾选多个平台授权）。填入后立即生效。
- **AI API Key（可选）**：[OpenAI](https://platform.openai.com/) 或 [通义千问](https://dashscope.aliyun.com/)，用于 AI 游记、润色与旁白生成。

### 2. 导入照片

- 工具栏点击「导入文件夹」或「选择图片」，支持 jpg / jpeg / png / tiff / webp
- 相同内容的照片自动去重（按文件 MD5）
- 导入时自动读取 EXIF（拍摄时间、GPS、相机型号）

### 3. 地图浏览

- 标记按缩放级别自动聚合，点击聚合查看照片列表
- 右下角切换标准 / 卫星地图，支持重置视图
- 「路线回放」按时间把照片连成线，重走旅程

### 4. 地点筛选

- 左侧地点树默认收起，点击「省」展开下一级，逐级下钻
- 点击任意地点：地图移动过去 + 照片列表联动筛选
- 没有地址的照片可通过「设置 → 批量逆地理编码」补全中文地址

### 5. 时间筛选

- 时间轴按天显示照片密度，拖动两端手柄选择日期区间
- 地图标记与照片列表立即过滤

### 6. 生成 AI 游记

- 「游记」页 → 选择 / 创建行程 → 「AI 生成游记」
- 生成后可在编辑器手动修改，或点击「AI 润色」
- 游记中 [photo:N] 标记的照片可点击跳转定位

### 7. 制作影集视频

- 「影集」页 → 选择照片 → 配置分辨率、时长、音乐 → 开始生成
- 需要系统已安装 ffmpeg（例如 `winget install ffmpeg`）

### 8. 语言与主题

- 设置 → 语言：7 种语言即时切换并自动保存
- 一键主题：多套配色实时预览

## 🔐 隐私说明

- 照片、数据库、缩略图全部保存在本机应用数据目录，无账号、无遥测
- 仅以下场景发起网络请求：
  - 地图瓦片与逆地理编码（高德，需你配置 Key）
  - AI 游记 / 旁白（你配置的 AI 服务商）
- API Key 经 Windows DPAPI 加密后落盘

## ❓ 常见问题

- **地图空白**：检查是否已配置高德 Key，且 Key 类型包含「Web端(JS API)」
- **地址识别失败**：检查「Web服务」Key 是否有效、当日配额是否用尽
- **影集生成失败**：安装 ffmpeg 并加入系统 PATH
- **海外使用**：当前地图依赖高德，海外区域显示受限，欢迎参与多地图源（OpenStreetMap / MapTiler 等）的开发

## 🤝 参与贡献

欢迎提交 Issue 与 Pull Request。开发前请先阅读 [docs/travel-journal-prd.md](docs/travel-journal-prd.md) 了解产品设计。

## 💖 打赏支持

如果 PhotoMap 对你有帮助，欢迎支持项目持续发展：

- 微信赞赏码（WeChat Reward）：

  ![微信赞赏码](docs/images/donate-qr.jpg)

## 💬 交流与反馈

- 问题与建议：[GitHub Issues](https://github.com/cuoduidui/photomap/issues)
- 邮箱：[780106788@qq.com](mailto:780106788@qq.com)
- 微信群 / QQ 群：待补充

## 📄 开源协议

本项目基于 [MIT License](LICENSE) 开源。

## 🗺️ Roadmap / 路线图

- [地图 Provider 抽象](docs/map-provider-abstraction.md)：解耦高德，支持 OpenStreetMap / MapTiler 等海外地图源（欢迎贡献）

---

## 🇬🇧 English

PhotoMap turns a folder of photos into an interactive travel map. It is a local-first Windows desktop app: photos are placed on a map by GPS, replayed as a route on the timeline, and summarized into an illustrated travel journal by AI. Everything stays on your device.

### Features

- Photo map with clustered markers, time/location filtering and route replay
- Location tree grouped by province → city → district → address; click a node to filter and fly to it
- Timeline density bar to filter photos and markers by date range
- AI travel journal in 7 languages, with manual editing and polishing
- Albums, photo collages and MP4 slideshows with background music and narration
- Custom tags and face grouping
- UI in 7 languages: 简体中文 / English / 日本語 / Français / 한국어 / Deutsch / Русский

### Tech stack

Vue 3 · Vite · Pinia · vue-i18n · Tauri 2 (Rust) · SQLite · AMap

### Prerequisites

Windows 10/11, Node.js 18+, Rust stable, ffmpeg (optional, for slideshows), and an AMap developer account.

### Quick start

```bash
npm install
npm run tauri dev
npm run tauri build
```

### First-run setup

Open Settings and fill in:

- **AMap API Key**: apply at the [AMap Open Platform](https://lbs.amap.com/). Authorize both "Web JS API" (map display) and "Web service" (reverse geocoding).
- **AI API Key (optional)**: [OpenAI](https://platform.openai.com/) or [Qwen](https://dashscope.aliyun.com/) for journal / narration generation.

### Privacy

Local-first. Photos and data never leave your device except map tiles / reverse geocoding (AMap) and AI requests (your configured provider). API keys are encrypted with Windows DPAPI.

### Roadmap

- [Map provider abstraction](docs/map-provider-abstraction.md): decouple AMap and add OpenStreetMap / MapTiler support for overseas users (contributions welcome)

### Donate & Contact

- WeChat reward QR:

  ![WeChat Reward](docs/images/donate-qr.jpg)
- Issues & suggestions: [GitHub Issues](https://github.com/cuoduidui/photomap/issues)
- Email: [780106788@qq.com](mailto:780106788@qq.com)
- Community group: TBD

### License

[MIT License](LICENSE)

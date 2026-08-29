# macOS 构建与打包验证清单

> 本文档用于在 macOS 上首次构建 PhotoMap 并逐项验证功能，确认跨平台适配无误。

## 1. 环境准备

```bash
# 1. 安装 Xcode Command Line Tools（编译器与签名工具）
xcode-select --install

# 2. 安装 Node.js 18+ 与 Rust（rustup 安装 stable 工具链）
node -v
rustc -V

# 3. 可选：安装 ffmpeg（影集视频导出需要）
brew install ffmpeg

# 4. 克隆并安装依赖
git clone git@github.com:cuoduidui/photomap.git
cd photomap
npm install
```

> 首次 `cargo` 构建会下载 macOS 专属依赖（`keyring`、`chacha20poly1305` 等），请保持网络畅通。

## 2. 构建

```bash
npm run tauri dev      # 开发模式（自动打开调试窗口）
npm run tauri build    # 正式构建
```

构建产物位于：

```text
src-tauri/target/release/bundle/
├── macos/PhotoMap.app
└── dmg/PhotoMap_1.0.0_x64.dmg
```

> 本地开发构建默认未签名。首次打开 .app 时，如果 Gatekeeper 拦截，
> 右键点击 App →「打开」，或在终端执行 `xattr -dr com.apple.quarantine PhotoMap.app`。

## 3. 打包验证清单

### 3.1 基础

- [ ] `npm run tauri build` 成功产出 `.app` 与 `.dmg`
- [ ] 双击 DMG，拖拽安装到「应用程序」后可以正常启动
- [ ] 启动后窗口尺寸/标题正常，无控制台报错
- [ ] 菜单栏、右键菜单无 Windows 残留（如 explorer/msiexec 字样）

### 3.2 密钥与安全存储

- [ ] 设置页可保存高德 Key 与 AI Key
- [ ] 数据库中的敏感配置带 `enc:v1:` 前缀（说明已加密而非明文）
  ```bash
  sqlite3 ~/Library/Application\ Support/com.cuoduidui.photomap/photomap.db \
    "SELECT key, substr(value,1,16) FROM app_config;"
  ```
- [ ] 系统钥匙串中存在 PhotoMap 主密钥条目（首次保存 Key 时可能弹出授权，点「始终允许」）
  ```bash
  security find-generic-password -s com.cuoduidui.photomap -w
  ```
- [ ] 重启应用后 Key 仍能正常读取（地图正常加载、逆地理编码可用）

### 3.3 地图与地点

- [ ] 高德地图正常加载（瓦片、缩放、拖拽）
- [ ] 导入带 GPS 的照片后出现地图标记，聚合数量与列表一致
- [ ] 地点树按 省/市/区县/街道/地址 折叠，点击节点地图自动移动
- [ ] 地点筛选显示中文地址，照片详情也显示中文地址
- [ ] 时间轴拖动后地图标记与照片列表同步过滤
- [ ] 路线回放：轨迹线、小车动画、播放/暂停/重播正常

> 注意：高德 Web JS API 在海外区域显示受限，属于已知限制（Roadmap 已规划多地图源）。

### 3.4 导出与影集

- [ ] 拼图/图片导出：标题文字正常渲染（macOS 使用 PingFang/STHeiti 字体，中文不乱码）
- [ ] 影集视频导出：ffmpeg 工作正常，成片含转场与音乐
- [ ] 右键「在 Finder 中显示」能定位到对应文件（使用 `open -R`）
- [ ] 导出目录选择对话框（Tauri dialog 插件）正常

### 3.5 AI 与数据

- [ ] AI 游记生成/润色正常（需配置 AI Key）
- [ ] 导入、批量逆地理编码、人脸分析可正常执行并支持取消
- [ ] 删除照片/标签后相关数据级联清理
- [ ] 应用数据位于 `~/Library/Application Support/com.cuoduidui.photomap/`，删除后可重置

### 3.6 界面与多语言

- [ ] 7 种语言切换正常（中文/English/日本語/Français/한국어/Deutsch/Русский）
- [ ] 明暗主题切换正常
- [ ] 窗口缩放、最小化、全屏无异常

## 4. 发布注意（面向未来上架/分发）

- 对外分发必须**签名**：`codesign --deep --force --sign "Developer ID Application: 你的证书" PhotoMap.app`
- 面向海外分发建议做**公证（notarization）**，避免用户看到「无法验证开发者」提示
- 若在 Mac App Store 上架，需要沙盒权限配置与 App Store 证书，属于单独工作项
- 打包签名/公证工具：`xcodebuild`、`codesign`、`notarytool`

## 5. 常见问题

| 问题 | 处理 |
| --- | --- |
| 首次保存 Key 时钥匙串弹窗 | 点「始终允许」，或到「钥匙串访问」中删除旧条目后重试 |
| 地图空白 | 检查高德 Key 是否已配置且类型为 Web JS API；海外网络受限时换 DNS 或等网络恢复 |
| 影集导出报「未找到 ffmpeg」 | `brew install ffmpeg`，确认 `which ffmpeg` 有输出 |
| 中文标题乱码 | 确认 `/System/Library/Fonts/PingFang.ttc` 存在（macOS 自带） |
| Gatekeeper 拦截未签名 App | 右键 → 打开；或 `xattr -dr com.apple.quarantine` 后重开 |

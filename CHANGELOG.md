# 更新日志 / Changelog

本项目所有重要变更都会记录在此文件中。
All notable changes to this project will be documented in this file.

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

## [Unreleased] / 未发布

### 🚧 计划中 / Planned

- 地图服务抽象层：接入 OpenStreetMap / MapTiler，支持海外用户（设计文档：[docs/map-provider-abstraction.md](docs/map-provider-abstraction.md)）
- macOS 支持：跨平台适配（Keychain 密钥保护、系统字体、Finder 定位）与构建文档
- macOS 打包验证清单（docs/build-macos.md）
- 补充应用截图与更多使用文档

## [1.0.0] - 2026-08-29

首个公开发布版本 / First public release。

### 🎉 新增 / Added

- 🗺️ 照片地图：按拍摄地点聚合展示，支持聚类缩放、地点/时间筛选与路线回放
- 📍 地点树：省/市/区县/街道/地址多级折叠筛选，点击节点地图自动定位
- ⏳ 时间轴：每日照片密度条与可拖拽日期范围，和地图标记、照片列表实时联动
- ✍️ AI 游记：自动生成图文游记，与照片双向锚定跳转
- 🖼️ 相册影集：相册、拼图与 MP4 幻灯片导出
- 🏷️ 标签与人脸：自定义标签与人脸分组
- 🌈 多语言：7 种界面语言，支持运行时切换（简体中文 / English / 日本語 / Français / 한국어 / Deutsch / Русский）
- 🔐 本地优先：照片与数据默认不离开设备

### 🐛 修复 / Fixed

- 地点筛选与照片详情展示中文地理地址（逆地理编码）
- WGS84→GCJ02 坐标统一转换，定位 / 聚类 / 回放坐标系一致
- 聚合标记数量与点开详情列表数量一致
- 点击地点节点地图自动移动并分级缩放
- 地点树默认折叠，点击节点才展开下一级
- 数据一致性：外键级联删除、游记计数同步、删除照片清理关联数据
- EXIF 方向统一处理：缩略图 / 导出 / 人脸检测
- Windows 混合路径分隔符下的文件删除问题
- 导入 / 批量逆地理编码 / 人脸分析支持随时取消

### 📦 其他 / Other

- Microsoft Store 提交素材与 MSIX 打包脚本（[scripts/package-msix.ps1](scripts/package-msix.ps1)）
- README 中英双语、MIT 开源协议、打赏与交流入口

### 版本对照 / Version Map

| 版本 | 发布日期 | 主要变化 |
| --- | --- | --- |
| 1.0.0 | 2026-08-29 | 首个公开发布：照片地图、地点树、时间轴、AI 游记、7 语言 |

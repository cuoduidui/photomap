# PhotoMap 微软商店（Microsoft Store）上架指南

本目录包含按微软商店要求整改所需的全部材料。代码层面的整改已完成：7 种界面语言（中/英/日/法/韩/德/俄）可在设置中切换并持久化；后端错误、AI 进度提示、AI 游记输出语言、自动生成的游记标题均跟随所选语言。

## 上架前清单（代码侧已完成）

- [x] 应用界面支持 7 种语言，设置内一键切换，自动保存
- [x] 后端错误信息翻译（不再向非中文用户展示中文错误）
- [x] AI 游记 / 润色 / 旁白按当前界面语言生成
- [x] 自动生成的游记标题按语言本地化
- [x] 打包目标包含 msi（Windows Store 转换 MSIX 的输入）
- [x] 安装包语言已配置（NSIS 简体中文 + 英文，Wix en-US）
- [ ] 隐私政策 URL（见 `privacy-policy.md`，需发布到公网）
- [ ] 支持邮箱（建议用 Outlook / 企业邮箱，填到 Partner Center）
- [ ] 应用图标 / 商店 Logo 与截图（见下文）

## 一、注册开发者账号

1. 打开 Partner Center（partner.microsoft.com/dashboard）→ 注册开发者账号。
2. 选择「个人开发者」：一次性费用 **$19**（约 ¥140）。
3. 按要求验证身份（中国大陆身份证或护照均可）。

## 二、收款设置（没有国外银行卡也可以）

1. Partner Center → 付款与税务 → 收款方式，选择 **PayPal**。
2. 注册 PayPal 国际账号，绑定国内银行卡即可收款（PayPal 可提现到国内银行卡，可能收取提现手续费）。
3. 填写税务信息：非美国开发者需填写 **W-8BEN** 表。中国大陆与美国有税收协定，通常适用 **0% 预扣税率**。
4. 微软每月 15 号左右结算上一周期收入，**起付金额 $50**（未满 $50 会累计到下一周期）。

## 三、打包 MSIX

微软商店只接受 **MSIX** 格式。Tauri 原生产出 MSI，需要转换一次：

**方式 A（推荐，无需额外 SDK）：**
1. 运行 `npm run tauri build`，在 `src-tauri/target/release/bundle/msi/` 得到 `.msi`。
2. 安装微软官方「MSIX Packaging Tool」（商店免费应用，搜索 "MSIX Packaging Tool"）。
3. 打开工具 → Package editor → 从现有安装程序创建包 → 选择 MSI → 按向导生成 `.msix`。

**方式 B（命令行自动化）：**
安装 Windows SDK 后运行：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/package-msix.ps1
```

脚本会在 `src-tauri/target/release/bundle/msix/` 输出 `PhotoMap_x.x.x.0_x64.msix`。

## 四、Partner Center 创建产品

1. **新建产品** → Windows 应用 → 保留名称（如 PhotoMap），填写包标识。
2. **上传包**：上传上面的 MSIX 文件。
3. **产品详情**：为 7 种语言各创建一个语言页，文案见 `listing-descriptions.md`。
4. **商店图片**：
   - Logo：至少 300×300 PNG（建议直接使用 `src-tauri/icons/512x512.png`）
   - 截图：至少 1 张，建议 4-8 张 1366×768 或 1920×1080（可在 `npm run tauri dev` 运行时截取各语言界面）
   - 推荐海报 / 宣传图可选
5. **隐私政策**：填写发布后的隐私政策 URL（模板见 `privacy-policy.md`）。
6. **支持联系信息**：填写支持邮箱与支持 URL。
7. **价格与可用性**：设为付费并填写价格（免费试用/试用期可选）。
8. **提交认证**：通常 1-3 个工作日，收到问题按邮件修改重新提交。

## 五、付费软件的注意事项

- 商店付费应用支持「试用 + 全功能购买」或直接买断。当前版本建议直接买断，先积累评价。
- 微软商店会抽成（开发者分成：应用收入前 $10,000/年抽 15%，超过后 12%）。
- 上架后需保持更新；如果应用长期不更新不会下架，但必须修复微软指出的兼容性问题。

## 六、当前仍建议的后续整改（海外可用性）

- **地图 Provider 抽象**：当前地图依赖高德（AMap），海外不可用。上架前建议增加可选的地图源（如 OpenStreetMap / MapTiler / Google Maps），否则海外用户地图空白。这是较大的独立改动，建议单独排期。
- **AI 服务商**：海外用户需自备 OpenAI API Key，现有界面已支持，无需改动。
- **ffmpeg 依赖**：影集视频功能依赖 ffmpeg，若未安装会给出已本地化的错误提示；上架说明/页面可注明。

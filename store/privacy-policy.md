# PhotoMap 隐私政策模板

> 微软商店要求应用必须提供「隐私政策 URL」，且必须能公网匿名访问（不要放在登录墙后）。本文件是模板，请替换其中占位内容后，发布到任意静态托管（GitHub Pages、Cloudflare Pages、Netlify、自己的网站等），把最终 URL 填入 Partner Center。

建议同时提供英文版（微软商店政策要求主要语言页为英文可读）。以下为中文与英文模板。

---

## 中文版

**PhotoMap 隐私政策**

生效日期：2026-XX-XX

PhotoMap（"本应用"）尊重你的隐私。本政策说明我们收集哪些信息、如何使用这些信息，以及你拥有的选择。

**1. 数据存储**
本应用是一个本地优先的桌面应用。你导入的照片、标签、游记等数据仅存储在你自己的设备上（应用数据目录与数据库文件）。本应用不注册账号、不搭建服务器、不上传你的照片。

**2. 网络请求**
- 地图与逆地理编码：当你启用地图功能并配置高德地图（AMap）API Key 时，本应用会将照片坐标发送给高德地图服务，用于显示地图与获取中文地址。API Key 由你自行申请并配置，存储在你设备本地（经 Windows DPAPI 加密）。
- AI 功能：当你配置 OpenAI / 通义千问等 AI 服务 API Key 后，生成游记、润色、生成旁白等功能会将游记文本与照片元数据（时间、地点、人物标签、拍摄设备）发送给你所选择的 AI 服务提供商。请在使用前确认该提供商的数据政策。
- 其他网络请求仅用于加载地图瓦片等必要内容。

**3. 数据共享**
除上述你主动配置的第三方服务（高德地图、AI 服务商）外，本应用不会向任何第三方出售、出租或共享你的数据。

**4. 数据删除**
删除照片或游记即从本设备删除对应数据。卸载应用后，应用数据目录中的残留文件可手动删除。

**5. 未成年人**
本应用不面向未成年人提供服务，也不会故意收集未成年人的个人信息。

**6. 政策变更**
本政策如有变更，我们会在应用更新说明中提示。

**7. 联系我们**
如有隐私相关问题，请发送邮件至：[你的支持邮箱，例如 support@example.com]

---

## English version

**PhotoMap Privacy Policy**

Effective date: 2026-XX-XX

PhotoMap ("the App") respects your privacy. This policy explains what information we collect, how we use it, and the choices you have.

**1. Data storage**
The App is a local-first desktop application. Photos, tags, travel journals and other data you import are stored only on your own device (app data directory and local database). The App has no account system, no servers, and does not upload your photos.

**2. Network requests**
- Map and reverse geocoding: When you enable the map feature and configure an AMap API Key, the App sends photo coordinates to AMap to render maps and obtain Chinese addresses. The API Key is provided by you, stored locally and encrypted with Windows DPAPI.
- AI features: When you configure an API Key for an AI provider (e.g. OpenAI, Qwen), journal generation, polishing and narration features send journal text and photo metadata (time, place, people tags, camera model) to the AI provider you selected. Please review that provider's data policy before use.
- Other network requests are limited to loading map tiles and similar necessary content.

**3. Data sharing**
Except for the third-party services you explicitly configure (AMap, AI providers), the App does not sell, rent or share your data with any third party.

**4. Data deletion**
Deleting a photo or journal removes the corresponding data from this device. After uninstalling, any residual files in the app data directory can be removed manually.

**5. Children**
The App is not directed to children and does not knowingly collect personal information from children.

**6. Policy changes**
If this policy changes, we will notify you in the app's release notes.

**7. Contact**
For privacy questions, email: [support email, e.g. support@example.com]

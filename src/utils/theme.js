import { getConfig, setConfig } from "./tauri";

// 皮肤/主题定义。preview 用于在设置里展示色块（取主色 + 辅色渐变）。
export const THEMES = [
  { id: "fresh", name: "清新薄荷", desc: "默认 · 薄荷绿 + 青蓝", preview: "linear-gradient(135deg, #18b88a, #1aa6c4)" },
  { id: "sky", name: "晴空蓝", desc: "天空蓝 + 湖青", preview: "linear-gradient(135deg, #3b9ae1, #4cc4d6)" },
  { id: "lavender", name: "薰衣草紫", desc: "柔紫 + 丁香", preview: "linear-gradient(135deg, #7c6cf0, #a06bf2)" },
  { id: "sunset", name: "暖阳橙", desc: "暖橙 + 珊瑚红", preview: "linear-gradient(135deg, #f0863a, #ef6f6f)" },
  { id: "dark", name: "暗夜墨", desc: "深色模式 · 墨绿 + 天蓝", preview: "linear-gradient(135deg, #2dd4a7, #38bdf8)" },
];

export const DEFAULT_THEME = "fresh";
export const THEME_KEY = "ui_theme";

export function isThemeId(id) {
  return THEMES.some((t) => t.id === id);
}

// 立即把主题应用到 <html data-theme="...">，切换无刷新、即时生效。
export function applyTheme(id) {
  const theme = isThemeId(id) ? id : DEFAULT_THEME;
  document.documentElement.setAttribute("data-theme", theme);
}

// 启动时调用：读取已保存的皮肤并应用；无记录则用默认。
export async function loadTheme() {
  let saved = null;
  try {
    saved = await getConfig(THEME_KEY);
  } catch (e) {
    console.warn("读取皮肤配置失败:", e);
  }
  const theme = isThemeId(saved) ? saved : DEFAULT_THEME;
  applyTheme(theme);
  return theme;
}

// 切换并持久化皮肤（一键换肤）。
export async function setTheme(id) {
  applyTheme(id);
  try {
    await setConfig(THEME_KEY, id);
  } catch (e) {
    console.warn("保存皮肤配置失败:", e);
  }
  return id;
}

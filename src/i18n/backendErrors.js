import i18n from "./index";

// 后端 Rust 返回的中文错误串 -> i18n key（精确匹配）
const EXACT = [
  ["未配置高德API Key", "backend.noAmapKey"],
  ["照片不存在", "backend.photoNotFound"],
  ["照片无GPS坐标", "backend.photoNoGps"],
  ["暂无已定位照片", "backend.noLocatedPhotos"],
  ["文件不存在", "backend.fileNotFound"],
  ["无权访问该路径", "backend.pathNotAllowed"],
  ["旅行不存在", "backend.tripNotFound"],
  ["请先写一些内容再润色", "backend.journalEmpty"],
  ["AI 未返回任何内容", "backend.aiNoContent"],
  ["没有照片", "backend.noPhotos"],
  ["所有照片文件不存在", "backend.allPhotosMissing"],
  ["没有可用的照片", "backend.noUsablePhotos"],
  ["DPAPI 加密失败", "backend.encryptFailed"],
  ["DPAPI 解密失败", "backend.decryptFailed"],
  ["应用数据目录包含非 UTF-8 字符，无法初始化数据库", "backend.dbPathInvalid"],
];

// 带错误详情后缀的（前缀匹配，保留原始 err 详情）
const PREFIX = [
  ["读取文件信息失败: ", "backend.readFileFailed", "err"],
  ["无法解析路径: ", "backend.pathResolveFailed", "err"],
  ["查询照片失败: ", "backend.queryPhotoFailed", "err"],
  ["请求失败: ", "backend.requestFailed", "err"],
  ["ffmpeg 编码失败: ", "backend.ffmpegEncodeFailed", "stderr"],
  ["ffmpeg 启动失败: ", "backend.ffmpegStartFailed", "err"],
  ["ffmpeg 处理图片失败: ", "backend.ffmpegImageFailed", "err"],
  ["视频合成失败: ", "backend.videoComposeFailed", "err"],
  ["创建输出目录失败: ", "backend.outputDirFailed", "err"],
  ["添加音乐失败: ", "backend.addMusicFailed", "err"],
  ["复制视频文件失败: ", "backend.copyVideoFailed", "err"],
  ["打开数据库失败: ", "backend.dbInitFailed", "err"],
];

const API_ERROR_RE = /^API 返回错误 \((\d+)\): ([\s\S]+)$/;

// AI 游记进度状态（Rust 事件文本）
const PROGRESS_EXACT = [
  ["正在分析旅行数据...", "trip.statusAnalyzing"],
  ["正在生成游记...", "trip.statusGeneratingJournal"],
  ["正在润色游记...", "trip.statusPolishingJournal"],
];

export function translateBackendError(err) {
  const s = String(err == null ? "" : err);
  if (!s) return err;
  for (const [match, key] of EXACT) {
    if (s === match) return i18n.global.t(key);
  }
  for (const [prefix, key, param] of PREFIX) {
    if (s.startsWith(prefix)) {
      return i18n.global.t(key, { [param]: s.slice(prefix.length) });
    }
  }
  const m = s.match(API_ERROR_RE);
  if (m) return i18n.global.t("backend.apiError", { status: m[1], text: m[2] });
  return err;
}

export function translateProgressText(text) {
  const s = String(text == null ? "" : text);
  for (const [match, key] of PROGRESS_EXACT) {
    if (s === match) return i18n.global.t(key);
  }
  return text;
}

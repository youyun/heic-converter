export type Locale = "en" | "zh";

export const LOCALES: { id: Locale; label: string }[] = [
  { id: "en", label: "English" },
  { id: "zh", label: "简体中文" },
];

const STORAGE_KEY = "heic-converter.locale";

type Dict = Record<string, string>;

const en: Dict = {
  "app.title": "HEIC Converter",
  "app.heading": "HEIC → JPG",
  "app.sub": "Bulk convert with resumable jobs — or drop a HEIC here",
  "toolbar.chooseFolder": "Choose folder",
  "toolbar.recursive": "Include subfolders",
  "toolbar.noFolder": "No folder selected",
  "options.customDest": "Custom destination folder",
  "options.pickDest": "Pick destination…",
  "options.defaultDest": "Default: JPG beside each HEIC",
  "options.deleteOriginals": "Delete original HEIC after successful convert",
  "list.selectAll": "Select all",
  "list.selected": "{n} selected",
  "list.convert": "Convert",
  "list.cancel": "Cancel",
  "job.label": "Job",
  "job.idle": "Idle",
  "job.emDash": "—",
  "job.progress": "{status} — {finished}/{total} completed ({pct}%)",
  "job.progressFailed": " · {n} failed",
  "job.nameFallback": "Convert {n} file(s) · {when}",
  "job.nameFolder": "{folder} · {when}",
  "job.started": "{name} started ({n} files).",
  "job.finished": "{name} finished: {status} ({finished}/{total})",
  "job.cancelRequested": "Cancel requested.",
  "confirm.title": "Start conversion?",
  "confirm.summary":
    "Convert {n} file(s) from “{folder}”. Confirm options, then start.",
  "confirm.folderFallback": "selected folder",
  "confirm.clear": "Clear",
  "confirm.cancel": "Cancel",
  "confirm.convert": "Convert",
  "confirm.noDestYet": "No destination selected yet",
  "dialog.chooseFolder": "Choose folder",
  "alert.needDest": "Pick a destination folder, or turn off custom destination.",
  "drop.offerFolder":
    "Open the parent folder?\n\n{parent}\n\nOK — browse / multi-select / bulk convert from that folder.\nCancel — select only the dropped photo(s).",
  "drop.hint": "Drop a HEIC/HEIF photo (or a folder) to open it.",
  "log.scanning": "Scanning {path}{recursive}…",
  "log.recursive": " (recursive)",
  "log.found": "Found {n} HEIC/HEIF file(s).",
  "log.autoOpen": "Auto-opening {path}",
  "log.loading": "Loading…",
  "log.fail": "FAIL {path}: {error}",
  "log.ok": "OK ({done}/{total}) {path}",
  "log.error": "Error: {msg}",
  "theme.light": "Light",
  "theme.dark": "Dark",
  "theme.system": "System",
  "theme.label": "Theme",
  "lang.label": "Language",
  "privacy.localOnly": "Files stay on this device — no upload.",
};

const zh: Dict = {
  "app.title": "HEIC 转换器",
  "app.heading": "HEIC → JPG",
  "app.sub": "批量转换，任务可恢复 — 也可把 HEIC 拖到这里",
  "toolbar.chooseFolder": "选择文件夹",
  "toolbar.recursive": "包含子文件夹",
  "toolbar.noFolder": "未选择文件夹",
  "options.customDest": "自定义输出文件夹",
  "options.pickDest": "选择输出位置…",
  "options.defaultDest": "默认：JPG 写在 HEIC 旁边",
  "options.deleteOriginals": "转换成功后删除原始 HEIC",
  "list.selectAll": "全选",
  "list.selected": "已选 {n} 个",
  "list.convert": "转换",
  "list.cancel": "取消",
  "job.label": "任务",
  "job.idle": "空闲",
  "job.emDash": "—",
  "job.progress": "{status} — 已完成 {finished}/{total}（{pct}%）",
  "job.progressFailed": " · 失败 {n}",
  "job.nameFallback": "转换 {n} 个文件 · {when}",
  "job.nameFolder": "{folder} · {when}",
  "job.started": "{name} 已开始（{n} 个文件）。",
  "job.finished": "{name} 结束：{status}（{finished}/{total}）",
  "job.cancelRequested": "已请求取消。",
  "confirm.title": "开始转换？",
  "confirm.summary":
    "将转换「{folder}」中的 {n} 个文件。请确认选项后开始。",
  "confirm.folderFallback": "所选文件夹",
  "confirm.clear": "清除",
  "confirm.cancel": "取消",
  "confirm.convert": "转换",
  "confirm.noDestYet": "尚未选择输出文件夹",
  "dialog.chooseFolder": "选择文件夹",
  "alert.needDest": "请选择输出文件夹，或关闭「自定义输出文件夹」。",
  "drop.offerFolder":
    "打开所在文件夹？\n\n{parent}\n\n确定 — 浏览 / 多选 / 批量转换该文件夹。\n取消 — 仅选中拖入的照片。",
  "drop.hint": "请拖入 HEIC/HEIF 照片或文件夹。",
  "log.scanning": "正在扫描 {path}{recursive}…",
  "log.recursive": "（含子文件夹）",
  "log.found": "找到 {n} 个 HEIC/HEIF 文件。",
  "log.autoOpen": "自动打开 {path}",
  "log.loading": "加载中…",
  "log.fail": "失败 {path}：{error}",
  "log.ok": "完成（{done}/{total}）{path}",
  "log.error": "错误：{msg}",
  "theme.light": "浅色",
  "theme.dark": "深色",
  "theme.system": "跟随系统",
  "theme.label": "主题",
  "lang.label": "语言",
  "privacy.localOnly": "仅在本机处理，不会上传。",
};

const catalogs: Record<Locale, Dict> = { en, zh };

let locale: Locale = "en";

export function detectInitialLocale(): Locale {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "en" || saved === "zh") return saved;
  } catch {
    /* ignore */
  }
  const nav = (navigator.language || "en").toLowerCase();
  return nav.startsWith("zh") ? "zh" : "en";
}

export function getLocale(): Locale {
  return locale;
}

export function setLocale(next: Locale) {
  locale = next;
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch {
    /* ignore */
  }
  document.documentElement.lang = next === "zh" ? "zh-CN" : "en";
}

export function t(key: string, vars?: Record<string, string | number>): string {
  const dict = catalogs[locale] ?? en;
  let s = dict[key] ?? en[key] ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      s = s.split(`{${k}}`).join(String(v));
    }
  }
  return s;
}

/** Apply `data-i18n` / `data-i18n-title` on static DOM. */
export function applyStaticI18n(root: ParentNode = document) {
  root.querySelectorAll<HTMLElement>("[data-i18n]").forEach((node) => {
    const key = node.dataset.i18n;
    if (!key) return;
    if (node.dataset.i18nHtml === "1") node.innerHTML = t(key);
    else node.textContent = t(key);
  });
  root.querySelectorAll<HTMLElement>("[data-i18n-title]").forEach((node) => {
    const key = node.dataset.i18nTitle;
    if (key) node.title = t(key);
  });
  document.title = t("app.title");
}

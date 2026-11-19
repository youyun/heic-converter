import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import {
  applyStaticI18n,
  detectInitialLocale,
  getLocale,
  setLocale,
  t,
  type Locale,
} from "./i18n";
import {
  detectInitialTheme,
  getThemePref,
  initTheme,
  setThemePref,
  type ThemePref,
} from "./theme";

type HeicEntry = {
  path: string;
  name: string;
  size_bytes: number;
  modified_ms: number | null;
};

type FolderScan = {
  root: string;
  total: number;
  entries: HeicEntry[];
};

type JobSummary = {
  id: string;
  status: string;
  dest_dir: string | null;
  delete_originals: boolean;
  total: number;
  done: number;
  failed: number;
  created_at: string;
};

type JobProgressEvent = {
  job_id: string;
  item_id: number;
  source_path: string;
  status: string;
  error: string | null;
  done: number;
  failed: number;
  total: number;
  job_status: string;
};

const ROW_HEIGHT = 32;
const OVERSCAN = 8;
const PAGE_SIZE = 200;

let folderPath: string | null = null;
let destPath: string | null = null;
let confirmDestPath: string | null = null;
let total = 0;
/** Sparse cache of loaded pages: index → entry */
const entries = new Map<number, HeicEntry>();
const selected = new Set<string>();
let currentJobId: string | null = null;
let currentJobName: string | null = null;
let loadingPages = new Set<number>();
let lastProgress: {
  done: number;
  failed: number;
  total: number;
  status: string;
} | null = null;
let appVersion = "—";
let pendingUpdate: Update | null = null;
let updateBusy = false;

const el = {
  pick: () => document.getElementById("btn-pick") as HTMLButtonElement,
  destBtn: () => document.getElementById("btn-dest") as HTMLButtonElement,
  convert: () => document.getElementById("btn-convert") as HTMLButtonElement,
  cancel: () => document.getElementById("btn-cancel") as HTMLButtonElement,
  recursive: () => document.getElementById("opt-recursive") as HTMLInputElement,
  optDest: () => document.getElementById("opt-dest") as HTMLInputElement,
  optDelete: () => document.getElementById("opt-delete") as HTMLInputElement,
  selectAll: () => document.getElementById("select-all") as HTMLInputElement,
  folderLabel: () => document.getElementById("folder-label") as HTMLElement,
  destLabel: () => document.getElementById("dest-label") as HTMLElement,
  totalCount: () => document.getElementById("total-count") as HTMLElement,
  selectedCount: () => document.getElementById("selected-count") as HTMLElement,
  viewport: () => document.getElementById("file-viewport") as HTMLElement,
  spacer: () => document.getElementById("file-spacer") as HTMLElement,
  window: () => document.getElementById("file-window") as HTMLElement,
  jobName: () => document.getElementById("job-name") as HTMLElement,
  jobId: () => document.getElementById("job-id") as HTMLElement,
  jobStatus: () => document.getElementById("job-status") as HTMLElement,
  progress: () => document.getElementById("progress-fill") as HTMLElement,
  log: () => document.getElementById("job-log") as HTMLElement,
  info: () => document.getElementById("converter-info") as HTMLElement,
  localeSelect: () =>
    document.getElementById("locale-select") as HTMLSelectElement,
  themeSelect: () =>
    document.getElementById("theme-select") as HTMLSelectElement,
  appVersion: () => document.getElementById("app-version") as HTMLElement,
  checkUpdate: () =>
    document.getElementById("btn-check-update") as HTMLButtonElement,
  installUpdate: () =>
    document.getElementById("btn-install-update") as HTMLButtonElement,
  updateStatus: () => document.getElementById("update-status") as HTMLElement,
  confirmBackdrop: () =>
    document.getElementById("confirm-backdrop") as HTMLElement,
  confirmSummary: () =>
    document.getElementById("confirm-summary") as HTMLElement,
  confirmDelete: () =>
    document.getElementById("confirm-delete") as HTMLInputElement,
  confirmDest: () =>
    document.getElementById("confirm-dest") as HTMLInputElement,
  confirmDestPick: () =>
    document.getElementById("confirm-dest-pick") as HTMLButtonElement,
  confirmDestClear: () =>
    document.getElementById("confirm-dest-clear") as HTMLButtonElement,
  confirmDestLabel: () =>
    document.getElementById("confirm-dest-label") as HTMLElement,
  confirmCancel: () =>
    document.getElementById("confirm-cancel") as HTMLButtonElement,
  confirmOk: () => document.getElementById("confirm-ok") as HTMLButtonElement,
};

function formatSize(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

function folderBaseName(path: string | null): string | null {
  if (!path) return null;
  const norm = path.replace(/\\/g, "/").replace(/\/+$/, "");
  const idx = norm.lastIndexOf("/");
  const base = idx >= 0 ? norm.slice(idx + 1) : norm;
  return base || null;
}

function formatJobTime(d = new Date()): string {
  const loc = getLocale() === "zh" ? "zh-CN" : undefined;
  return d.toLocaleString(loc, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/** Human-readable job title; UUID stays secondary. */
function makeJobName(fileCount: number): string {
  const folder = folderBaseName(folderPath);
  const when = formatJobTime();
  if (folder) return t("job.nameFolder", { folder, when });
  return t("job.nameFallback", { n: fileCount, when });
}

function logLine(msg: string) {
  const box = el.log();
  const loc = getLocale() === "zh" ? "zh-CN" : undefined;
  const time = new Date().toLocaleTimeString(loc);
  box.textContent = `${time}  ${msg}\n` + (box.textContent ?? "");
}

function updateSelectionUi() {
  el.selectedCount().textContent = t("list.selected", { n: selected.size });
  el.convert().disabled = selected.size === 0 || !folderPath;
  el.selectAll().checked = total > 0 && selected.size === total;
  el.selectAll().indeterminate = selected.size > 0 && selected.size < total;
}

function setProgress(
  done: number,
  failed: number,
  totalCount: number,
  status: string,
) {
  lastProgress = { done, failed, total: totalCount, status };
  const finished = done + failed;
  const pct = totalCount > 0 ? Math.round((finished / totalCount) * 100) : 0;
  el.progress().style.width = `${pct}%`;
  const failBit =
    failed > 0 ? t("job.progressFailed", { n: failed }) : "";
  el.jobStatus().textContent =
    t("job.progress", {
      status,
      finished,
      total: totalCount,
      pct,
    }) + failBit;
}

function setJobLabel(name: string, id: string) {
  currentJobName = name;
  el.jobName().textContent = name;
  el.jobId().textContent = id.slice(0, 8);
  el.jobId().title = id;
  el.jobId().hidden = false;
}

function syncToolbarDestLabel() {
  if (el.optDest().checked && destPath) {
    el.destLabel().textContent = destPath;
    el.destLabel().title = destPath;
  } else {
    el.destLabel().textContent = t("options.defaultDest");
    el.destLabel().title = "";
  }
}

function syncConfirmDestUi() {
  const useDest = el.confirmDest().checked;
  el.confirmDestPick().disabled = !useDest;
  el.confirmDestClear().disabled = !useDest || !confirmDestPath;
  if (useDest && confirmDestPath) {
    el.confirmDestLabel().textContent = confirmDestPath;
    el.confirmDestLabel().title = confirmDestPath;
  } else if (useDest) {
    el.confirmDestLabel().textContent = t("confirm.noDestYet");
    el.confirmDestLabel().title = "";
  } else {
    el.confirmDestLabel().textContent = t("options.defaultDest");
    el.confirmDestLabel().title = "";
  }
}

function syncFolderLabel() {
  if (folderPath) {
    el.folderLabel().textContent = folderPath;
    el.folderLabel().title = folderPath;
  } else {
    el.folderLabel().textContent = t("toolbar.noFolder");
    el.folderLabel().title = "";
  }
}

function setUpdateStatus(text: string) {
  el.updateStatus().textContent = text;
}

function syncUpdateUi() {
  el.appVersion().textContent = t("update.version", { version: appVersion });
  el.installUpdate().hidden = !pendingUpdate;
  el.checkUpdate().disabled = updateBusy;
  el.installUpdate().disabled = updateBusy || !pendingUpdate;
}

function refreshLocalizedChrome() {
  applyStaticI18n();
  el.localeSelect().value = getLocale();
  el.themeSelect().value = getThemePref();
  syncFolderLabel();
  syncToolbarDestLabel();
  syncUpdateUi();
  if (!pendingUpdate && !updateBusy) {
    setUpdateStatus(t("update.idle"));
  } else if (pendingUpdate && !updateBusy) {
    setUpdateStatus(t("update.available", { version: pendingUpdate.version }));
  }
  if (!el.confirmBackdrop().hidden) {
    const n = selected.size;
    const folder =
      folderBaseName(folderPath) ?? t("confirm.folderFallback");
    el.confirmSummary().textContent = t("confirm.summary", { n, folder });
    syncConfirmDestUi();
  } else {
    syncConfirmDestUi();
  }
  updateSelectionUi();
  if (lastProgress) {
    setProgress(
      lastProgress.done,
      lastProgress.failed,
      lastProgress.total,
      lastProgress.status,
    );
  } else if (!currentJobId) {
    el.jobStatus().textContent = t("job.idle");
    if (!currentJobName) el.jobName().textContent = t("job.emDash");
  }
}

async function checkForUpdates() {
  if (updateBusy) return;
  updateBusy = true;
  pendingUpdate = null;
  syncUpdateUi();
  setUpdateStatus(t("update.checking"));
  try {
    const update = await check();
    if (!update) {
      setUpdateStatus(t("update.upToDate", { version: appVersion }));
      return;
    }
    pendingUpdate = update;
    setUpdateStatus(t("update.available", { version: update.version }));
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    setUpdateStatus(t("update.error", { msg }));
    logLine(t("update.error", { msg }));
  } finally {
    updateBusy = false;
    syncUpdateUi();
  }
}

async function installPendingUpdate() {
  if (!pendingUpdate || updateBusy) return;
  const version = pendingUpdate.version;
  if (!window.confirm(t("update.confirmInstall", { version }))) return;

  updateBusy = true;
  syncUpdateUi();
  let downloaded = 0;
  let contentLength: number | undefined;
  try {
    await pendingUpdate.downloadAndInstall((event) => {
      if (event.event === "Started") {
        contentLength = event.data.contentLength;
        setUpdateStatus(t("update.downloading", { pct: 0 }));
      } else if (event.event === "Progress") {
        downloaded += event.data.chunkLength;
        const pct =
          contentLength && contentLength > 0
            ? Math.min(99, Math.round((downloaded / contentLength) * 100))
            : 0;
        setUpdateStatus(t("update.downloading", { pct }));
      } else if (event.event === "Finished") {
        setUpdateStatus(t("update.installing"));
      }
    });
    setUpdateStatus(t("update.installing"));
    await relaunch();
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    setUpdateStatus(t("update.error", { msg }));
    logLine(t("update.error", { msg }));
    updateBusy = false;
    syncUpdateUi();
  }
}

function openConfirmDialog() {
  const n = selected.size;
  const folder = folderBaseName(folderPath) ?? t("confirm.folderFallback");
  el.confirmSummary().textContent = t("confirm.summary", { n, folder });
  el.confirmDelete().checked = el.optDelete().checked;
  el.confirmDest().checked = el.optDest().checked;
  confirmDestPath = el.optDest().checked ? destPath : null;
  syncConfirmDestUi();
  el.confirmBackdrop().hidden = false;
  el.confirmOk().focus();
}

function closeConfirmDialog() {
  el.confirmBackdrop().hidden = true;
}

async function ensureRange(start: number, end: number) {
  const pageStart = Math.floor(start / PAGE_SIZE);
  const pageEnd = Math.floor(Math.max(end - 1, start) / PAGE_SIZE);
  const tasks: Promise<void>[] = [];
  for (let p = pageStart; p <= pageEnd; p++) {
    const offset = p * PAGE_SIZE;
    let need = false;
    for (let i = offset; i < Math.min(offset + PAGE_SIZE, total); i++) {
      if (!entries.has(i)) {
        need = true;
        break;
      }
    }
    if (!need || loadingPages.has(p)) continue;
    loadingPages.add(p);
    tasks.push(
      invoke<FolderScan>("list_heic_page", { offset, limit: PAGE_SIZE })
        .then((page) => {
          page.entries.forEach((entry, i) => entries.set(offset + i, entry));
        })
        .finally(() => loadingPages.delete(p)),
    );
  }
  if (tasks.length) await Promise.all(tasks);
}

async function renderVisible() {
  const viewport = el.viewport();
  const scrollTop = viewport.scrollTop;
  const height = viewport.clientHeight;
  const start = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
  const visible = Math.ceil(height / ROW_HEIGHT) + OVERSCAN * 2;
  const end = Math.min(total, start + visible);

  await ensureRange(start, end);

  el.spacer().style.height = `${total * ROW_HEIGHT}px`;
  const win = el.window();
  win.style.transform = `translateY(${start * ROW_HEIGHT}px)`;

  const frag = document.createDocumentFragment();
  for (let i = start; i < end; i++) {
    const entry = entries.get(i);
    const row = document.createElement("div");
    row.className = "file-row";
    const cb = document.createElement("input");
    cb.type = "checkbox";
    if (entry) {
      cb.checked = selected.has(entry.path);
      cb.addEventListener("change", () => {
        if (cb.checked) selected.add(entry.path);
        else selected.delete(entry.path);
        updateSelectionUi();
      });
      const name = document.createElement("div");
      name.className = "name";
      name.textContent = entry.name;
      name.title = entry.path;
      const size = document.createElement("div");
      size.className = "size";
      size.textContent = formatSize(entry.size_bytes);
      row.append(cb, name, size);
    } else {
      cb.disabled = true;
      const name = document.createElement("div");
      name.className = "name";
      name.textContent = t("log.loading");
      row.append(cb, name, document.createElement("div"));
    }
    frag.appendChild(row);
  }
  win.replaceChildren(frag);
}

async function loadFolder(path: string, preselect?: string[]) {
  folderPath = path;
  entries.clear();
  selected.clear();
  syncFolderLabel();
  const recursive = el.recursive().checked;
  logLine(
    t("log.scanning", {
      path,
      recursive: recursive ? t("log.recursive") : "",
    }),
  );
  const scan = await invoke<FolderScan>("scan_folder", { path, recursive });
  total = scan.total;
  scan.entries.forEach((e, i) => entries.set(i, e));
  if (preselect && preselect.length > 0) {
    const want = new Set(preselect.map(normalizePath));
    for (const entry of entries.values()) {
      if (want.has(normalizePath(entry.path))) selected.add(entry.path);
    }
  }
  el.totalCount().textContent = String(total);
  updateSelectionUi();
  await renderVisible();
  logLine(t("log.found", { n: total }));
}

function normalizePath(p: string): string {
  return p.replace(/\\/g, "/").toLowerCase();
}

function isHeicPath(p: string): boolean {
  return /\.(heic|heif|hif)$/i.test(p);
}

function parentDir(p: string): string {
  const norm = p.replace(/\\/g, "/");
  const idx = norm.lastIndexOf("/");
  return idx <= 0 ? p : p.slice(0, idx);
}

async function handleDroppedPaths(paths: string[]) {
  if (!paths.length) return;
  const first = paths[0];

  // Directory drop → open that folder.
  if (paths.length === 1 && (await invoke<boolean>("path_is_dir", { path: first }))) {
    await loadFolder(first);
    return;
  }

  const heics = paths.filter(isHeicPath);
  if (heics.length === 0) {
    logLine(t("drop.hint"));
    return;
  }

  const parent = parentDir(heics[0]);
  const openFolder = window.confirm(t("drop.offerFolder", { parent }));

  if (openFolder) {
    await loadFolder(parent);
  } else {
    await loadFolder(parent, heics);
  }
}

/** Native folder dialog via the JS plugin (non-blocking). Never use a Rust blocking_pick_* command. */
async function pickDirectory(): Promise<string | null> {
  // With directory:true + multiple:false this resolves to string | null.
  return await open({
    directory: true,
    multiple: false,
    title: t("dialog.chooseFolder"),
  });
}

async function pickFolder() {
  const path = await pickDirectory();
  if (path) await loadFolder(path);
}

async function pickDest() {
  const path = await pickDirectory();
  if (!path) return;
  destPath = path;
  el.optDest().checked = true;
  el.destBtn().disabled = false;
  syncToolbarDestLabel();
}

async function pickConfirmDest() {
  const path = await pickDirectory();
  if (!path) return;
  confirmDestPath = path;
  el.confirmDest().checked = true;
  syncConfirmDestUi();
}

function clearConfirmDest() {
  confirmDestPath = null;
  syncConfirmDestUi();
}

async function selectAllVisible(checked: boolean) {
  if (!checked) {
    selected.clear();
    updateSelectionUi();
    await renderVisible();
    return;
  }
  // Load all pages then select — for thousands of files this is path-only.
  for (let offset = 0; offset < total; offset += PAGE_SIZE) {
    await ensureRange(offset, Math.min(offset + PAGE_SIZE, total));
  }
  for (const entry of entries.values()) selected.add(entry.path);
  updateSelectionUi();
  await renderVisible();
}

async function startConvert(opts?: {
  deleteOriginals: boolean;
  useDest: boolean;
  destDir: string | null;
  skipConfirm?: boolean;
}) {
  if (selected.size === 0) return;

  if (!opts?.skipConfirm) {
    openConfirmDialog();
    return;
  }

  const deleteOriginals = opts.deleteOriginals;
  const useDest = opts.useDest;
  const destDir = useDest ? opts.destDir : null;
  if (useDest && !destDir) {
    alert(t("alert.needDest"));
    return;
  }

  // Keep toolbar options in sync with what was confirmed.
  el.optDelete().checked = deleteOriginals;
  el.optDest().checked = useDest;
  el.destBtn().disabled = !useDest;
  destPath = useDest ? destDir : null;
  syncToolbarDestLabel();

  const paths = Array.from(selected);
  const jobName = makeJobName(paths.length);
  const job = await invoke<JobSummary>("create_convert_job", {
    paths,
    destDir,
    deleteOriginals,
  });
  currentJobId = job.id;
  setJobLabel(jobName, job.id);
  setProgress(0, 0, job.total, "running");
  el.convert().disabled = true;
  el.cancel().disabled = false;
  logLine(t("job.started", { name: jobName, n: job.total }));
  await invoke("start_convert_job", { jobId: job.id });
}

async function confirmAndStart() {
  const useDest = el.confirmDest().checked;
  const destDir = useDest ? confirmDestPath : null;
  if (useDest && !destDir) {
    alert(t("alert.needDest"));
    return;
  }
  closeConfirmDialog();
  await startConvert({
    deleteOriginals: el.confirmDelete().checked,
    useDest,
    destDir,
    skipConfirm: true,
  });
}

window.addEventListener("DOMContentLoaded", async () => {
  setLocale(detectInitialLocale());
  initTheme(detectInitialTheme());
  refreshLocalizedChrome();

  el.localeSelect().addEventListener("change", () => {
    setLocale(el.localeSelect().value as Locale);
    refreshLocalizedChrome();
    void renderVisible();
  });
  el.themeSelect().addEventListener("change", () => {
    setThemePref(el.themeSelect().value as ThemePref);
  });
  el.checkUpdate().addEventListener("click", () => {
    void checkForUpdates();
  });
  el.installUpdate().addEventListener("click", () => {
    void installPendingUpdate();
  });

  el.pick().addEventListener("click", () => void pickFolder().catch(showErr));
  el.destBtn().addEventListener("click", () => void pickDest().catch(showErr));
  el.convert().addEventListener("click", () => void startConvert().catch(showErr));
  el.cancel().addEventListener("click", () => {
    void invoke("cancel_convert_job").then(() =>
      logLine(t("job.cancelRequested")),
    );
  });
  el.optDest().addEventListener("change", () => {
    el.destBtn().disabled = !el.optDest().checked;
    if (!el.optDest().checked) {
      destPath = null;
    }
    syncToolbarDestLabel();
  });
  el.selectAll().addEventListener("change", () => {
    void selectAllVisible(el.selectAll().checked).catch(showErr);
  });
  el.viewport().addEventListener("scroll", () => {
    void renderVisible();
  });
  window.addEventListener("resize", () => {
    void renderVisible();
  });

  el.confirmCancel().addEventListener("click", () => closeConfirmDialog());
  el.confirmOk().addEventListener("click", () => {
    void confirmAndStart().catch(showErr);
  });
  el.confirmDest().addEventListener("change", () => {
    if (!el.confirmDest().checked) confirmDestPath = null;
    syncConfirmDestUi();
  });
  el.confirmDestPick().addEventListener("click", () => {
    void pickConfirmDest().catch(showErr);
  });
  el.confirmDestClear().addEventListener("click", () => clearConfirmDest());
  el.confirmBackdrop().addEventListener("click", (ev) => {
    if (ev.target === el.confirmBackdrop()) closeConfirmDialog();
  });
  window.addEventListener("keydown", (ev) => {
    if (ev.key === "Escape" && !el.confirmBackdrop().hidden) {
      closeConfirmDialog();
    }
  });

  await listen<JobProgressEvent>("job-progress", (event) => {
    const p = event.payload;
    if (currentJobId && p.job_id !== currentJobId) return;
    setProgress(p.done, p.failed, p.total, p.job_status);
    if (p.status === "job_finished") {
      el.cancel().disabled = true;
      updateSelectionUi();
      const label = currentJobName ?? t("job.label");
      logLine(
        t("job.finished", {
          name: label,
          status: p.job_status,
          finished: p.done + p.failed,
          total: p.total,
        }),
      );
      return;
    }
    if (p.status === "failed") {
      logLine(
        t("log.fail", { path: p.source_path, error: p.error ?? "" }),
      );
    } else if (p.status === "done" && p.item_id >= 0) {
      // Keep log light — only every 10th success or first.
      if (p.done === 1 || p.done % 10 === 0) {
        logLine(
          t("log.ok", {
            done: p.done,
            total: p.total,
            path: p.source_path,
          }),
        );
      }
    }
  });

  try {
    const unlisten = await getCurrentWebview().onDragDropEvent((event) => {
      const appRoot = document.querySelector(".app");
      if (event.payload.type === "enter" || event.payload.type === "over") {
        appRoot?.classList.add("drag-active");
        return;
      }
      appRoot?.classList.remove("drag-active");
      if (event.payload.type === "drop") {
        void handleDroppedPaths(event.payload.paths).catch(showErr);
      }
    });
    window.addEventListener("unload", () => unlisten());
  } catch {
    /* webview drag-drop unavailable outside Tauri */
  }

  try {
    appVersion = await getVersion();
  } catch {
    appVersion = "—";
  }
  syncUpdateUi();
  setUpdateStatus(t("update.idle"));

  try {
    const info = await invoke<{
      heif_dec: string | null;
      bundled_heif_dec: string | null;
      libheif: boolean;
      sips: boolean;
      platform: string;
    }>("converter_info");
    const bits = [info.platform];
    if (info.libheif) bits.push("libheif (built-in)");
    if (info.bundled_heif_dec) bits.push("bundled heif-dec");
    else if (info.heif_dec) bits.push("heif-dec");
    if (info.sips) bits.push("sips");
    el.info().textContent = bits.join(" · ");
  } catch {
    /* ignore */
  }

  try {
    const auto = await invoke<string | null>("auto_open_folder");
    if (auto) {
      logLine(t("log.autoOpen", { path: auto }));
      await loadFolder(auto);
      const flag = await invoke<string>("auto_convert_flag");
      if (flag.trim() === "1") {
        await selectAllVisible(true);
        // Headless/smoke path: skip the modal and use toolbar options.
        await startConvert({
          deleteOriginals: el.optDelete().checked,
          useDest: el.optDest().checked,
          destDir: el.optDest().checked ? destPath : null,
          skipConfirm: true,
        });
      }
    }
  } catch (err) {
    showErr(err);
  }
});

function showErr(err: unknown) {
  const msg = err instanceof Error ? err.message : String(err);
  logLine(t("log.error", { msg }));
  alert(msg);
}

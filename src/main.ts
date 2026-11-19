import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

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
let total = 0;
/** Sparse cache of loaded pages: index → entry */
const entries = new Map<number, HeicEntry>();
const selected = new Set<string>();
let currentJobId: string | null = null;
let loadingPages = new Set<number>();

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
  jobId: () => document.getElementById("job-id") as HTMLElement,
  jobStatus: () => document.getElementById("job-status") as HTMLElement,
  progress: () => document.getElementById("progress-fill") as HTMLElement,
  log: () => document.getElementById("job-log") as HTMLElement,
  info: () => document.getElementById("converter-info") as HTMLElement,
};

function formatSize(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

function logLine(msg: string) {
  const box = el.log();
  const t = new Date().toLocaleTimeString();
  box.textContent = `${t}  ${msg}\n` + (box.textContent ?? "");
}

function updateSelectionUi() {
  el.selectedCount().textContent = `${selected.size} selected`;
  el.convert().disabled = selected.size === 0 || !folderPath;
  el.selectAll().checked = total > 0 && selected.size === total;
  el.selectAll().indeterminate = selected.size > 0 && selected.size < total;
}

function setProgress(done: number, failed: number, totalCount: number, status: string) {
  const finished = done + failed;
  const pct = totalCount > 0 ? Math.round((finished / totalCount) * 100) : 0;
  el.progress().style.width = `${pct}%`;
  el.jobStatus().textContent =
    `${status} — ${done} done, ${failed} failed / ${totalCount} (${pct}%)`;
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
      name.textContent = "Loading…";
      row.append(cb, name, document.createElement("div"));
    }
    frag.appendChild(row);
  }
  win.replaceChildren(frag);
}

async function loadFolder(path: string) {
  folderPath = path;
  entries.clear();
  selected.clear();
  el.folderLabel().textContent = path;
  el.folderLabel().title = path;
  const recursive = el.recursive().checked;
  logLine(`Scanning ${path}${recursive ? " (recursive)" : ""}…`);
  const scan = await invoke<FolderScan>("scan_folder", { path, recursive });
  total = scan.total;
  scan.entries.forEach((e, i) => entries.set(i, e));
  el.totalCount().textContent = String(total);
  updateSelectionUi();
  await renderVisible();
  logLine(`Found ${total} HEIC/HEIF file(s).`);
}

/** Native folder dialog via the JS plugin (non-blocking). Never use a Rust blocking_pick_* command. */
async function pickDirectory(): Promise<string | null> {
  // With directory:true + multiple:false this resolves to string | null.
  return await open({
    directory: true,
    multiple: false,
    title: "Choose folder",
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
  el.destLabel().textContent = path;
  el.destLabel().title = path;
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

async function startConvert() {
  if (selected.size === 0) return;
  const useDest = el.optDest().checked;
  const paths = Array.from(selected);
  const job = await invoke<JobSummary>("create_convert_job", {
    paths,
    destDir: useDest ? destPath : null,
    deleteOriginals: el.optDelete().checked,
  });
  currentJobId = job.id;
  el.jobId().textContent = job.id.slice(0, 8);
  setProgress(0, 0, job.total, "running");
  el.convert().disabled = true;
  el.cancel().disabled = false;
  logLine(`Job ${job.id.slice(0, 8)} started (${job.total} files).`);
  await invoke("start_convert_job", { jobId: job.id });
}

window.addEventListener("DOMContentLoaded", async () => {
  el.pick().addEventListener("click", () => void pickFolder().catch(showErr));
  el.destBtn().addEventListener("click", () => void pickDest().catch(showErr));
  el.convert().addEventListener("click", () => void startConvert().catch(showErr));
  el.cancel().addEventListener("click", () => {
    void invoke("cancel_convert_job").then(() => logLine("Cancel requested."));
  });
  el.optDest().addEventListener("change", () => {
    el.destBtn().disabled = !el.optDest().checked;
    if (!el.optDest().checked) {
      destPath = null;
      el.destLabel().textContent = "Default: JPG beside each HEIC";
    }
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

  await listen<JobProgressEvent>("job-progress", (event) => {
    const p = event.payload;
    if (currentJobId && p.job_id !== currentJobId) return;
    setProgress(p.done, p.failed, p.total, p.job_status);
    if (p.status === "job_finished") {
      el.cancel().disabled = true;
      updateSelectionUi();
      logLine(`Job finished: ${p.job_status}`);
      return;
    }
    if (p.status === "failed") {
      logLine(`FAIL ${p.source_path}: ${p.error ?? ""}`);
    } else if (p.status === "done" && p.item_id >= 0) {
      // Keep log light — only every 10th success or first.
      if (p.done === 1 || p.done % 10 === 0) {
        logLine(`OK (${p.done}/${p.total}) ${p.source_path}`);
      }
    }
  });

  try {
    const info = await invoke<{
      heif_dec: string | null;
      sips: boolean;
      platform: string;
    }>("converter_info");
    const bits = [info.platform];
    if (info.heif_dec) bits.push("heif-dec");
    if (info.sips) bits.push("sips");
    el.info().textContent = bits.join(" · ");
  } catch {
    /* ignore */
  }

  try {
    const auto = await invoke<string | null>("auto_open_folder");
    if (auto) {
      logLine(`Auto-opening ${auto}`);
      await loadFolder(auto);
      const flag = await invoke<string>("auto_convert_flag");
      if (flag.trim() === "1") {
        await selectAllVisible(true);
        await startConvert();
      }
    }
  } catch (err) {
    showErr(err);
  }
});

function showErr(err: unknown) {
  const msg = err instanceof Error ? err.message : String(err);
  logLine(`Error: ${msg}`);
  alert(msg);
}

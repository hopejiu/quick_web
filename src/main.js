import {invoke} from "@tauri-apps/api/core";

// ---- DOM ----
const $ = (id) => document.getElementById(id);
const siteList = $("siteList");
const addEntryBtn = $("addEntryBtn");
const autoStartToggle = $("autoStartToggle");
const startMinimizedToggle = $("startMinimizedToggle");
const saveStatus = $("saveStatus");
const siteModal = $("siteModal");
const modalTitle = $("modalTitle");
const entryName = $("entryName");
const entryProtocol = $("entryProtocol");
const entryUrl = $("entryUrl");
const entryKeySelect = $("entryKeySelect");
const entryScriptDir = $("entryScriptDir");
const openScriptDirBtn = $("openScriptDirBtn");
const hotkeyDisplay = $("hotkeyDisplay");
const hotkeyConflict = $("hotkeyConflict");
const cancelEntryBtn = $("cancelEntryBtn");
const saveEntryBtn = $("saveEntryBtn");

let settings = null;
let editingId = null; // null = 新增，非 null = 编辑
let selectedMod = "Alt";

// ---- 工具函数 ----
function showSaveStatus(msg, type = "success") {
  saveStatus.textContent = msg;
  saveStatus.className = "save-status " + type;
  setTimeout(() => { saveStatus.className = "save-status"; }, 2000);
}

async function doSave() {
  if (!settings) return;
  try {
    await invoke("save_settings", { data: settings });
    showSaveStatus("已保存");
  } catch (err) {
    showSaveStatus("保存失败: " + (err?.message || String(err)), "error");
  }
}

function genId() {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

function deriveDirFromUrl(url) {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return "unknown";
  }
}

// ---- 站点列表渲染 ----
function renderSiteList() {
  const entries = settings.entries || [];
  if (entries.length === 0) {
    siteList.innerHTML = '<div class="site-empty">暂无站点，点击"添加站点"开始</div>';
    return;
  }
  siteList.innerHTML = entries.map(e => `
    <div class="site-item" data-id="${e.id}">
      <div class="site-info">
        <div class="site-name">${escHtml(e.name)}</div>
        <div class="site-url">${escHtml(e.url)}</div>
        <div class="site-meta">
          <span class="badge badge-startup">${escHtml(e.hotkey || '无快捷键')}</span>
          <span style="font-size:11px;color:var(--text-secondary);">脚本: ${escHtml(e.script_dir)}</span>
        </div>
      </div>
      <div class="site-actions">
        <button class="icon-btn" title="打开脚本目录" onclick="openScriptDir('${escAttr(e.script_dir)}')">📁</button>
        <button class="icon-btn" title="编辑" onclick="editEntry('${escAttr(e.id)}')">✏️</button>
        ${e.id !== 'default' ? `<button class="icon-btn danger" title="删除" onclick="deleteEntry('${escAttr(e.id)}')">🗑️</button>` : ''}
      </div>
    </div>
  `).join('');
}

function escHtml(s) {
  const d = document.createElement('div');
  d.textContent = s;
  return d.innerHTML;
}

function escAttr(s) {
  return s.replace(/'/g, "\\'").replace(/"/g, "&quot;");
}

// ---- 弹窗逻辑 ----
function openModal(entry) {
  editingId = entry ? entry.id : null;
  modalTitle.textContent = entry ? "编辑站点" : "添加站点";
  entryName.value = entry ? entry.name : "";

  // 拆分协议和域名
  const fullUrl = entry ? entry.url : "";
  if (fullUrl.startsWith("http://")) {
    entryProtocol.value = "http://";
    entryUrl.value = fullUrl.slice(7);
  } else {
    entryProtocol.value = "https://";
    entryUrl.value = fullUrl.replace(/^https:\/\//, "");
  }

  entryScriptDir.value = entry ? entry.script_dir : "";

  // 解析快捷键
  const hk = entry ? entry.hotkey : "";
  const parts = hk.split("+");
  selectedMod = parts[0] || "Alt";
  entryKeySelect.value = parts.slice(1).join("+") || "";
  updateModButtons();
  updateHotkeyPreview();
  updateKeyOptions();
  clearConflict();

  siteModal.classList.add("active");
}

function closeModal() {
  siteModal.classList.remove("active");
  editingId = null;
}

function updateModButtons() {
  document.querySelectorAll("#modBtns .hotkey-btn").forEach(btn => {
    btn.classList.toggle("selected", btn.dataset.mod === selectedMod);
  });
}

function updateHotkeyPreview() {
  const key = entryKeySelect.value;
  hotkeyDisplay.textContent = key ? selectedMod + "+" + key : "未设置";
}

function clearConflict() {
  hotkeyConflict.textContent = "";
}

function getUsedHotkeys() {
  const entries = settings.entries || [];
  const used = new Set();
  for (const e of entries) {
    if (e.id !== editingId && e.hotkey) {
      used.add(e.hotkey);
    }
  }
  return used;
}

function validateHotkey(hk) {
  if (!hk) return null;
  const used = getUsedHotkeys();
  if (used.has(hk)) {
    return `快捷键 ${hk} 已被其他站点占用`;
  }
  return null;
}

// ---- 快捷键选项过滤：禁用已被占用的按键 ----
function updateKeyOptions() {
  const used = getUsedHotkeys();
  const opts = entryKeySelect.querySelectorAll("option");
  opts.forEach(opt => {
    if (!opt.value) return; // skip placeholder
    const hk = selectedMod + "+" + opt.value;
    if (used.has(hk)) {
      opt.disabled = true;
      opt.textContent = opt.value + " (已占用)";
    } else {
      opt.disabled = false;
      opt.textContent = opt.value;
    }
  });
  // 如果当前选中的被禁用了，清空选择
  const currentKey = entryKeySelect.value;
  if (currentKey) {
    const currentHk = selectedMod + "+" + currentKey;
    if (used.has(currentHk)) {
      entryKeySelect.value = "";
      updateHotkeyPreview();
    }
  }
}

// ---- URL 变化时自动更新脚本目录（始终自动推导） ----
entryUrl.addEventListener("input", () => {
  updateScriptDirFromUrl();
});

entryProtocol.addEventListener("change", () => {
  updateScriptDirFromUrl();
});

function updateScriptDirFromUrl() {
  const domain = entryUrl.value.trim();
  if (!domain) {
    entryScriptDir.value = "";
    return;
  }
  const fullUrl = entryProtocol.value + domain;
  try {
    const u = new URL(fullUrl);
    entryScriptDir.value = u.hostname.replace(/^www\./, "");
  } catch {
    // 域名不合法时保留上一次的值
  }
}

// ---- 快捷键选择 ----
document.querySelectorAll("#modBtns .hotkey-btn").forEach(btn => {
  btn.addEventListener("click", () => {
    selectedMod = btn.dataset.mod;
    updateModButtons();
    updateKeyOptions();
    updateHotkeyPreview();
    clearConflict();
    checkCurrentHotkey();
  });
});

entryKeySelect.addEventListener("change", () => {
  updateHotkeyPreview();
  clearConflict();
  checkCurrentHotkey();
});

async function checkCurrentHotkey() {
  const key = entryKeySelect.value;
  if (!key) return;
  const hk = selectedMod + "+" + key;
  try {
    await invoke("check_hotkey", { hotkey: hk, editingId: editingId });
    clearConflict();
  } catch (err) {
    hotkeyConflict.textContent = "⚠ " + (err?.message || String(err));
  }
}

// ---- 弹窗事件 ----
addEntryBtn.addEventListener("click", () => openModal(null));
cancelEntryBtn.addEventListener("click", closeModal);
siteModal.addEventListener("click", (e) => { if (e.target === siteModal) closeModal(); });

openScriptDirBtn.addEventListener("click", async () => {
  const dir = entryScriptDir.value.trim();
  if (!dir) return;
  try {
    await invoke("open_script_dir", { dirName: dir });
  } catch (err) {
    showSaveStatus("打开目录失败: " + (err?.message || String(err)), "error");
  }
});

saveEntryBtn.addEventListener("click", async () => {
  const name = entryName.value.trim();
  const domain = entryUrl.value.trim();
  const protocol = entryProtocol.value;
  const key = entryKeySelect.value;
  const scriptDir = entryScriptDir.value.trim();

  if (!name) { showSaveStatus("请输入名称", "error"); return; }
  if (!domain) { showSaveStatus("请输入网址", "error"); return; }
  if (!key) { showSaveStatus("请选择快捷键", "error"); return; }

  const url = protocol + domain;
  const hk = selectedMod + "+" + key;
  const conflict = validateHotkey(hk);
  if (conflict) {
    hotkeyConflict.textContent = conflict;
    showSaveStatus(conflict, "error");
    return;
  }

  const entry = {
    id: editingId || genId(),
    url,
    name,
    hotkey: hk,
    script_dir: scriptDir || deriveDirFromUrl(url)
  };

  // 更新 entries
  if (editingId) {
    const idx = settings.entries.findIndex(e => e.id === editingId);
    if (idx >= 0) settings.entries[idx] = entry;
  } else {
    settings.entries.push(entry);
  }

  await doSave();
  renderSiteList();
  closeModal();
});

// ---- 全局操作（由 onclick 调用） ----
window.openScriptDir = async (dirName) => {
  try {
    await invoke("open_script_dir", { dirName });
  } catch (err) {
    showSaveStatus("打开目录失败: " + (err?.message || String(err)), "error");
  }
};

window.editEntry = (id) => {
  const entry = (settings.entries || []).find(e => e.id === id);
  if (entry) openModal(entry);
};

// ---- 确认弹窗 ----
const confirmModal = $("confirmModal");
const confirmTitle = $("confirmTitle");
const confirmDesc = $("confirmDesc");
const confirmCancelBtn = $("confirmCancelBtn");
const confirmOkBtn = $("confirmOkBtn");

function showConfirm(title, desc) {
  return new Promise(resolve => {
    confirmTitle.textContent = title;
    confirmDesc.textContent = desc;
    confirmModal.classList.add("active");
    const close = (result) => {
      confirmModal.classList.remove("active");
      confirmOkBtn.onclick = null;
      confirmCancelBtn.onclick = null;
      resolve(result);
    };
    confirmOkBtn.onclick = () => close(true);
    confirmCancelBtn.onclick = () => close(false);
    confirmModal.onclick = (e) => { if (e.target === confirmModal) close(false); };
  });
}

window.deleteEntry = async (id) => {
  const entry = (settings.entries || []).find(e => e.id === id);
  const name = entry ? entry.name : "此站点";
  const ok = await showConfirm("删除站点", `确定删除「${name}」？删除后不可恢复。`);
  if (!ok) return;
  settings.entries = (settings.entries || []).filter(e => e.id !== id);
  await doSave();
  renderSiteList();
};

// ---- 开关事件 ----
autoStartToggle.addEventListener("change", async () => {
  settings.auto_start = autoStartToggle.checked;
  await doSave();
});

startMinimizedToggle.addEventListener("change", async () => {
  settings.start_minimized = startMinimizedToggle.checked;
  await doSave();
});

// ---- 初始化 ----
async function init() {
  settings = await invoke("get_settings");
  console.log("[settings] loaded:", JSON.stringify(settings, null, 2));

  // 确保 entries 是数组
  if (!Array.isArray(settings.entries)) settings.entries = [];

  console.log("[settings] entries:", settings.entries.length);
  settings.entries.forEach((e, i) => {
    console.log(`[settings] entry[${i}]:`, e.id, e.url, e.hotkey, e.script_dir);
  });

  renderSiteList();
  autoStartToggle.checked = settings.auto_start;
  startMinimizedToggle.checked = settings.start_minimized;
}

document.addEventListener("DOMContentLoaded", init);

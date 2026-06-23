import { Service as SettingsService } from "../bindings/changeme/internal/settings";

// ---- DOM 引用 ----
const $ = (id) => document.getElementById(id);
const modSelect = $("modSelect");
const keySelect = $("keySelect");
const hotkeyDisplay = $("hotkeyDisplay");
const hotkeyError = $("hotkeyError");
const saveStatus = $("saveStatus");
const autoStartToggle = $("autoStartToggle");

let settings = null;

// ---- 工具函数 ----
function showSaveStatus(msg, type = "success") {
  saveStatus.textContent = msg;
  saveStatus.className = "save-status " + type;
  setTimeout(() => { saveStatus.className = "save-status"; }, 2000);
}

function showHotkeyError(msg) {
  hotkeyError.textContent = msg;
  hotkeyError.className = "error-msg visible";
  setTimeout(() => { hotkeyError.className = "error-msg"; }, 4000);
}

// ---- 保存设置 ----
async function doSave() {
  if (!settings) return;
  try {
    await SettingsService.SaveSettings(settings);
    showSaveStatus("已保存");
  } catch (err) {
    const msg = err?.message || String(err);
    showSaveStatus("保存失败: " + msg, "error");
  }
}

// ---- 快捷键变更 ----
async function onHotkeyChange() {
  const mod = modSelect.value;
  const key = keySelect.value;
  if (!key) { showHotkeyError("请选择一个按键"); return; }

  const hotkey = mod + "+" + key;
  hotkeyDisplay.textContent = hotkey;

  const oldHk = settings.hotkey;
  settings.hotkey = hotkey;

  try {
    await doSave();
    hotkeyError.className = "error-msg";
  } catch (err) {
    // 热键注册失败（冲突）
    settings.hotkey = oldHk;
    const msg = err?.message || String(err);
    showHotkeyError("快捷键冲突: " + msg);
    const parts = oldHk.split("+");
    modSelect.value = parts[0] || "Alt";
    keySelect.value = parts.slice(1).join("+") || "";
    hotkeyDisplay.textContent = oldHk;
  }
}

modSelect.addEventListener("change", onHotkeyChange);
keySelect.addEventListener("change", onHotkeyChange);

// ---- 开机自启 ----
autoStartToggle.addEventListener("change", async () => {
  settings.auto_start = autoStartToggle.checked;
  await doSave();
});

// ---- 初始化 ----
async function init() {
  settings = await SettingsService.GetSettings();

  // 解析热键
  const parts = settings.hotkey.split("+");
  modSelect.value = parts[0] || "Alt";
  keySelect.value = parts.slice(1).join("+") || "";
  hotkeyDisplay.textContent = settings.hotkey;

  // 开机自启
  autoStartToggle.checked = settings.auto_start;
}

document.addEventListener("DOMContentLoaded", init);

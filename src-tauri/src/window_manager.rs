use std::collections::HashSet;
use std::sync::Mutex;

/// 窗口关闭协调器。
///
/// 站点窗口关闭分两条路径：
/// 1. 用户点 X：on_window_event 里 prevent_close + hide + 启动定时器
/// 2. 定时器触发：emit 事件 → 标记 → win.close() → 第二次 CloseRequested
///
/// WindowManager 区分这两条路径：第二条路径检查 take_closing() 时返回 true，
/// 让 on_window_event 不调用 prevent_close，允许窗口真正关闭。
#[derive(Debug)]
pub struct WindowManager {
    pending_close: Mutex<HashSet<String>>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self { pending_close: Mutex::new(HashSet::new()) }
    }

    /// 标记某个窗口为"定时器要求销毁"（将在下一次 CloseRequested 中被允许）
    pub fn mark_closing(&self, label: &str) {
        self.pending_close.lock().unwrap().insert(label.to_string());
    }

    /// 检查并清除标记（on_window_event 调用）
    /// 返回 true = 本次关闭由定时器触发，不应用 prevent_close
    pub fn take_closing(&self, label: &str) -> bool {
        self.pending_close.lock().unwrap().remove(label)
    }
}

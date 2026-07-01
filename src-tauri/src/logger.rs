/// 文件日志系统（按小时轮转，72h 保留）
///
/// 与原 deepseek-pc Go 版 internal/log/log.go 功能等价：
/// - 日志目录: {config_dir}/logs/
/// - 文件名: app-YYYY-MM-DD-HH.log
/// - 每小时轮转，保留 72 小时
/// - DevMode 时同时输出到 stderr

use chrono::Local;
use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

struct LoggerInner {
    current_name: String,
    file: Option<File>,
    dir: PathBuf,
}

pub struct FileLogger {
    inner: Mutex<LoggerInner>,
    dev_mode: bool,
}

fn log_file_name() -> String {
    format!("app-{}.log", Local::now().format("%Y-%m-%d-%H"))
}

/// 初始化日志系统。dev_mode=true 时同时输出到 stderr。
pub fn init(log_dir: PathBuf, dev_mode: bool) -> Result<(), SetLoggerError> {
    fs::create_dir_all(&log_dir).ok();
    let name = log_file_name();
    let path = log_dir.join(&name);
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok();

    let logger = FileLogger {
        inner: Mutex::new(LoggerInner {
            current_name: name,
            file,
            dir: log_dir,
        }),
        dev_mode,
    };

    log::set_boxed_logger(Box::new(logger))
        .map(|()| log::set_max_level(LevelFilter::Debug))
}

impl Log for FileLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let mut inner = self.inner.lock().unwrap();
        let ts = Local::now().format("%Y-%m-%dT%H:%M:%S%.3f");

        // ---- 轮转检查 ----
        let name = log_file_name();
        if inner.current_name != name {
            if let Some(ref mut f) = inner.file {
                let _ = f.flush();
            }
            let path = inner.dir.join(&name);
            inner.file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .ok();
            inner.current_name = name.clone();
            clean_old_logs(&inner.dir, &name);
        }

        // ---- 写文件 ----
        let line = format!("{} [{}] {}\n", ts, record.level(), record.args());
        if let Some(ref mut file) = inner.file {
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();
        }

        // ---- DevMode stderr ----
        if self.dev_mode {
            let _ = eprint!("{}", line);
        }
    }

    fn flush(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            if let Some(ref mut file) = inner.file {
                let _ = file.flush();
            }
        }
    }
}

/// 清理 72 小时前的旧日志
fn clean_old_logs(dir: &PathBuf, current: &str) {
    let cutoff = Local::now() - chrono::Duration::hours(72);
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".log") || name == current {
                continue;
            }
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    let modified: chrono::DateTime<Local> = modified.into();
                    if modified < cutoff {
                        fs::remove_file(entry.path()).ok();
                    }
                }
            }
        }
    }
}

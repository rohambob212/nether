use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// A single log line flowing from the engine to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogRecord {
    pub ts_ms: u64,
    /// lowercase rust log level: error | warn | info | debug | trace
    pub level: String,
    pub target: String,
    pub message: String,
}

impl LogRecord {
    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    fn new(level: log::Level, target: &str, message: &str) -> Self {
        Self {
            ts_ms: Self::now_ms(),
            level: level.to_string().to_lowercase(),
            target: target.to_string(),
            message: message.to_string(),
        }
    }
}

/// Broadcasts every engine log line to subscribers (the UI and the status
/// watcher) while keeping a bounded in-memory history for late joiners.
pub struct LogHub {
    tx: tokio::sync::broadcast::Sender<LogRecord>,
    history: std::sync::Mutex<Vec<LogRecord>>,
    cap: usize,
}

static HUB: std::sync::OnceLock<LogHub> = std::sync::OnceLock::new();

fn init_hub(capacity: usize) -> LogHub {
    LogHub {
        tx: tokio::sync::broadcast::channel(4096).0,
        history: std::sync::Mutex::new(Vec::with_capacity(1024)),
        cap: capacity.max(100),
    }
}

/// Install the global logger that feeds the hub. Must run before the Aether
/// library initializes its own env_logger (its `try_init` then becomes a
/// no-op, so all of its `log` output lands in our hub instead of stderr).
pub fn install(capacity: usize) -> &'static LogHub {
    let hub = HUB.get_or_init(|| init_hub(capacity));

    let _ = log::set_boxed_logger(Box::new(HubLogger));
    log::set_max_level(log::LevelFilter::Trace);
    hub
}

/// Access the hub after [`install`].
pub fn hub() -> &'static LogHub {
    HUB.get_or_init(|| init_hub(5000))
}

impl LogHub {
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<LogRecord> {
        self.tx.subscribe()
    }

    pub fn snapshot(&self, limit: usize) -> Vec<LogRecord> {
        let hist = self.history.lock().unwrap_or_else(|p| p.into_inner());
        if hist.len() <= limit {
            hist.clone()
        } else {
            hist[hist.len() - limit..].to_vec()
        }
    }

    pub fn clear(&self) {
        self.history.lock().unwrap_or_else(|p| p.into_inner()).clear();
    }

    fn push(&self, rec: LogRecord) {
        {
            let mut hist = self.history.lock().unwrap_or_else(|p| p.into_inner());
            if hist.len() >= self.cap {
                let overflow = hist.len() + 1 - self.cap;
                hist.drain(..overflow);
            }
            hist.push(rec.clone());
        }
        let _ = self.tx.send(rec);
    }
}

struct HubLogger;

impl log::Log for HubLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        // Capture everything; filtering happens in the UI.
        metadata.level() <= log::Level::Trace
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let rec = LogRecord::new(record.level(), record.target(), &record.args().to_string());
        hub().push(rec);
    }

    fn flush(&self) {}
}

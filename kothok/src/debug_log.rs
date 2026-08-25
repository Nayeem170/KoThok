use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::data::config::DEBUG_LOG;
use crate::BUILD_TAG;

const WEB3FORMS_URL: &str = "https://api.web3forms.com/submit";
const WEB3FORMS_KEY: &str = "651486d1-94e9-43df-8d78-56b6b47b8e7b";
const CHECK_INTERVAL_SECS: u64 = 60;

static PENDING: OnceLock<String> = OnceLock::new();
static DEVICE_MODEL: OnceLock<String> = OnceLock::new();
static UPLOAD_QUEUED: AtomicBool = AtomicBool::new(false);
static LAST_UPLOAD_ATTEMPT: AtomicU64 = AtomicU64::new(0);

pub fn log(msg: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(DEBUG_LOG)
    {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let _ = writeln!(f, "[{ts}] {msg}");
        let _ = f.sync_data();
    }
}

pub fn check_on_startup(device_model: &str) {
    let _ = DEVICE_MODEL.set(device_model.to_string());

    let content = std::fs::read_to_string(DEBUG_LOG).unwrap_or_default();
    if content.trim().is_empty() {
        return;
    }

    log::info!("debug-log: previous debug log found, will upload when WiFi connects");
    let _ = PENDING.set(content);
    UPLOAD_QUEUED.store(true, Ordering::Relaxed);
}

pub fn try_upload_if_wifi() {
    if !UPLOAD_QUEUED.load(Ordering::Relaxed) {
        return;
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if now.saturating_sub(LAST_UPLOAD_ATTEMPT.load(Ordering::Relaxed)) < CHECK_INTERVAL_SECS {
        return;
    }

    if !crate::device::wifi_status() {
        return;
    }

    LAST_UPLOAD_ATTEMPT.store(now, Ordering::Relaxed);

    let Some(content) = PENDING.get() else {
        return;
    };
    let content = content.clone();
    let device = DEVICE_MODEL
        .get()
        .cloned()
        .unwrap_or_else(|| "Unknown".into());

    std::thread::spawn(move || {
        if upload(&device, &content) {
            UPLOAD_QUEUED.store(false, Ordering::Relaxed);
        }
    });
}

fn upload(device: &str, content: &str) -> bool {
    let body = serde_json::json!({
        "access_key": WEB3FORMS_KEY,
        "subject": format!("KoThok debug log - {BUILD_TAG}"),
        "from_name": device,
        "Build": BUILD_TAG,
        "message": content,
    });

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(30))
        .build();

    match agent
        .post(WEB3FORMS_URL)
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
    {
        Ok(resp) => {
            let success = resp
                .into_string()
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .and_then(|v| v.get("success").and_then(|s| s.as_bool()))
                .unwrap_or(false);
            if success {
                log::info!("debug-log: uploaded, clearing debug.log");
                let _ = std::fs::write(DEBUG_LOG, "");
            } else {
                log::warn!("debug-log: server rejected upload");
            }
            success
        }
        Err(e) => {
            log::warn!("debug-log: upload failed: {e}");
            false
        }
    }
}

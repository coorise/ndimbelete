//! Optional LISTEN/NOTIFY watcher for remote pushes.
//!
//! Note: `postgres::Socket` is only the TCP stream type — realtime alerts use
//! `NOTIFY ndimbelente_push` + a dedicated listener connection.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use postgres::fallible_iterator::FallibleIterator;
use tauri::{AppHandle, Emitter};

use super::remote::open_remote;

pub const REMOTE_PUSH_EVENT: &str = "collab://remote-push";

static LISTEN_STOP: AtomicBool = AtomicBool::new(false);
static LISTEN_URI: Mutex<Option<String>> = Mutex::new(None);
static APP_HANDLE: Mutex<Option<AppHandle>> = Mutex::new(None);
static LISTENER_STARTED: AtomicBool = AtomicBool::new(false);

pub fn set_app_handle(app: AppHandle) {
    *APP_HANDLE.lock().unwrap_or_else(|e| e.into_inner()) = Some(app);
}

pub fn start_listener(uri: String) {
    {
        let mut guard = LISTEN_URI.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(uri);
    }
    LISTEN_STOP.store(false, Ordering::SeqCst);

    if LISTENER_STARTED.swap(true, Ordering::SeqCst) {
        return; // already running; URI updated for next reconnect
    }

    thread::spawn(|| {
        loop {
            if LISTEN_STOP.load(Ordering::SeqCst) {
                LISTENER_STARTED.store(false, Ordering::SeqCst);
                break;
            }

            let uri = LISTEN_URI
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();

            let Some(uri) = uri else {
                thread::sleep(Duration::from_secs(2));
                continue;
            };

            match listen_once(&uri) {
                Ok(()) => {}
                Err(_) => {
                    // Free-tier sleep / network blip — retry after delay.
                    thread::sleep(Duration::from_secs(15));
                }
            }
        }
    });
}

pub fn stop_listener() {
    LISTEN_STOP.store(true, Ordering::SeqCst);
    *LISTEN_URI.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

fn listen_once(uri: &str) -> anyhow::Result<()> {
    let mut client = open_remote(uri)?;
    client.batch_execute("LISTEN ndimbelente_push")?;

    loop {
        if LISTEN_STOP.load(Ordering::SeqCst) {
            break;
        }
        let mut notifications = client.notifications();
        if let Some(n) = notifications
            .timeout_iter(Duration::from_secs(5))
            .next()?
        {
            let payload = n.payload().to_string();
            if let Some(app) = APP_HANDLE.lock().unwrap_or_else(|e| e.into_inner()).clone() {
                let _ = app.emit(REMOTE_PUSH_EVENT, payload);
            }
        }
        // Refresh URI in case it changed.
        let current = LISTEN_URI
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if current.as_deref() != Some(uri) {
            break; // reconnect with new URI
        }
    }
    Ok(())
}

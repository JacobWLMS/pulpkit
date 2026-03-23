use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::icons::resolve_icon_from_cache;
use crate::state::TrayItem;

pub fn start_tray_watcher(
    tray_items: Arc<std::sync::Mutex<Vec<TrayItem>>>,
    dirty_flag: Arc<AtomicBool>,
    icon_cache: HashMap<String, String>,
    activate_rx: tokio::sync::mpsc::Receiver<(String, String)>,
) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        rt.block_on(async {
            let Ok(client) = system_tray::client::Client::new().await else {
                eprintln!("[pulpkit] tray client failed to start");
                return;
            };
            let mut rx = client.subscribe();
            let mut activate_rx = activate_rx;

            // Build initial items
            {
                let items = client.items().lock().unwrap().clone();
                let mut tray = tray_items.lock().unwrap();
                tray.clear();
                for (addr, (item, _menu)) in &items {
                    let id = item.id.clone();
                    let address = addr.clone();
                    let title = item.title.clone().unwrap_or_default();
                    let icon_name = item.icon_name.clone().unwrap_or_default();
                    let icon = if !icon_name.is_empty() {
                        resolve_icon_from_cache(&icon_name, &icon_cache)
                    } else {
                        String::new()
                    };
                    tray.push(TrayItem {
                        id,
                        address,
                        title,
                        icon,
                    });
                }
                dirty_flag.store(true, Ordering::Relaxed);
            }

            loop {
                tokio::select! {
                    ev = rx.recv() => {
                        if ev.is_err() { break; }
                        let items = client.items().lock().unwrap().clone();
                        let mut tray = tray_items.lock().unwrap();
                        tray.clear();
                        for (addr, (item, _menu)) in &items {
                            let id = item.id.clone();
                            let address = addr.clone();
                            let title = item.title.clone().unwrap_or_default();
                            let icon_name = item.icon_name.clone().unwrap_or_default();
                            let icon = if !icon_name.is_empty() {
                                resolve_icon_from_cache(&icon_name, &icon_cache)
                            } else { String::new() };
                            tray.push(TrayItem { id, address, title, icon });
                        }
                        dirty_flag.store(true, Ordering::Relaxed);
                    }
                    Some((address, click)) = activate_rx.recv() => {
                        use system_tray::client::ActivateRequest;
                        let req = match click.as_str() {
                            "right" => ActivateRequest::Secondary { address, x: 0, y: 0 },
                            _ => ActivateRequest::Default { address, x: 0, y: 0 },
                        };
                        if let Err(e) = client.activate(req).await {
                            eprintln!("[pulpkit] tray activate error: {e}");
                        }
                    }
                }
            }
        });
    });
}

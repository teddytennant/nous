use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Manager,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub did: String,
    pub peers: u32,
    pub uptime_secs: u64,
    pub version: String,
    pub modules: Vec<ModuleStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleStatus {
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    pub token: String,
    pub balance: String,
    pub usd_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: String,
    pub encrypted: bool,
}

#[tauri::command]
fn get_node_status() -> NodeStatus {
    NodeStatus {
        did: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".into(),
        peers: 0,
        uptime_secs: 0,
        version: env!("CARGO_PKG_VERSION").into(),
        modules: vec![
            ModuleStatus { name: "Identity".into(), status: "active".into() },
            ModuleStatus { name: "Messaging".into(), status: "active".into() },
            ModuleStatus { name: "Governance".into(), status: "active".into() },
            ModuleStatus { name: "Social".into(), status: "active".into() },
            ModuleStatus { name: "Payments".into(), status: "standby".into() },
            ModuleStatus { name: "Storage".into(), status: "active".into() },
            ModuleStatus { name: "AI".into(), status: "standby".into() },
            ModuleStatus { name: "Browser".into(), status: "standby".into() },
        ],
    }
}

#[tauri::command]
fn get_wallet_balances() -> Vec<WalletBalance> {
    vec![
        WalletBalance { token: "ETH".into(), balance: "0.000".into(), usd_value: Some("$0.00".into()) },
        WalletBalance { token: "NOUS".into(), balance: "0.000".into(), usd_value: None },
        WalletBalance { token: "USDC".into(), balance: "0.000".into(), usd_value: Some("$0.00".into()) },
    ]
}

#[tauri::command]
fn get_recent_messages() -> Vec<Message> {
    vec![]
}

#[tauri::command]
fn get_identity() -> serde_json::Value {
    serde_json::json!({
        "did": "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
        "keys": [
            { "type": "ed25519", "purpose": "signing", "fingerprint": "z6Mkh...2doK" },
            { "type": "x25519", "purpose": "exchange", "fingerprint": "z6LSb...9xKn" }
        ],
        "credentials": []
    })
}

#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let show = MenuItem::with_id(app, "show", "Show Nous", true, None::<&str>)?;
    let status = MenuItem::with_id(app, "status", "Status: Online", false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &status, &quit])?;

    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Nous — Sovereign Protocol")
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            get_node_status,
            get_wallet_balances,
            get_recent_messages,
            get_identity,
            app_version,
        ])
        .setup(|app| {
            setup_tray(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run nous desktop");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_status_has_all_modules() {
        let status = get_node_status();
        assert_eq!(status.modules.len(), 8);
        assert!(!status.did.is_empty());
    }

    #[test]
    fn wallet_balances_default() {
        let balances = get_wallet_balances();
        assert_eq!(balances.len(), 3);
        assert_eq!(balances[0].token, "ETH");
    }

    #[test]
    fn recent_messages_empty() {
        let messages = get_recent_messages();
        assert!(messages.is_empty());
    }

    #[test]
    fn identity_has_did() {
        let identity = get_identity();
        assert!(identity["did"].as_str().unwrap().starts_with("did:key:"));
    }

    #[test]
    fn version_not_empty() {
        let v = app_version();
        assert!(!v.is_empty());
    }

    #[test]
    fn node_status_serializes() {
        let status = get_node_status();
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: NodeStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.did, status.did);
    }

    #[test]
    fn wallet_balance_serializes() {
        let balances = get_wallet_balances();
        let json = serde_json::to_string(&balances).unwrap();
        let deserialized: Vec<WalletBalance> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 3);
    }
}

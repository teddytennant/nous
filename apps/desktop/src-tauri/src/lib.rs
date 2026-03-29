use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use nous_terminal::{Cell as TermCell, Color as TermColor, RenderRow, Terminal, TerminalConfig};
use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Manager, State,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tokio::sync::RwLock;
use uuid::Uuid;

const DEFAULT_API_URL: &str = "http://localhost:8080/api/v1";

pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
    identity_did: RwLock<Option<String>>,
}

impl ApiClient {
    fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: DEFAULT_API_URL.to_string(),
            identity_did: RwLock::new(None),
        }
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        self.client
            .get(format!("{}{}", self.base_url, path))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<T>()
            .await
            .map_err(|e| e.to_string())
    }

    async fn post<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &impl Serialize,
    ) -> Result<T, String> {
        self.client
            .post(format!("{}{}", self.base_url, path))
            .json(body)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<T>()
            .await
            .map_err(|e| e.to_string())
    }
}

// ── Types ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub did: String,
    pub peers: u32,
    pub uptime_secs: u64,
    pub version: String,
    pub modules: Vec<ModuleStatus>,
    pub api_connected: bool,
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
pub struct IdentityInfo {
    pub did: String,
    pub display_name: Option<String>,
    pub signing_key_type: String,
    pub exchange_key_type: String,
}

// API response types
#[derive(Debug, Deserialize)]
struct HealthResponse {
    status: String,
    version: String,
    uptime_ms: u64,
}

#[derive(Debug, Deserialize)]
struct WalletResponse {
    balances: Vec<BalanceEntry>,
}

#[derive(Debug, Deserialize)]
struct BalanceEntry {
    token: String,
    amount: String,
}

// ── Commands ──────────────────────────────────────────────────────────────

#[tauri::command]
async fn get_node_status(api: State<'_, Arc<ApiClient>>) -> Result<NodeStatus, String> {
    match api.get::<HealthResponse>("/health").await {
        Ok(health) => {
            let did = api
                .identity_did
                .read()
                .await
                .clone()
                .unwrap_or_else(|| "not initialized".into());
            Ok(NodeStatus {
                did,
                peers: 0,
                uptime_secs: health.uptime_ms / 1000,
                version: health.version,
                modules: vec![
                    ModuleStatus {
                        name: "Identity".into(),
                        status: "active".into(),
                    },
                    ModuleStatus {
                        name: "Messaging".into(),
                        status: "active".into(),
                    },
                    ModuleStatus {
                        name: "Governance".into(),
                        status: "active".into(),
                    },
                    ModuleStatus {
                        name: "Social".into(),
                        status: "active".into(),
                    },
                    ModuleStatus {
                        name: "Payments".into(),
                        status: "active".into(),
                    },
                    ModuleStatus {
                        name: "Storage".into(),
                        status: "active".into(),
                    },
                    ModuleStatus {
                        name: "AI".into(),
                        status: "standby".into(),
                    },
                    ModuleStatus {
                        name: "Browser".into(),
                        status: "standby".into(),
                    },
                ],
                api_connected: health.status == "ok",
            })
        }
        Err(_) => Ok(NodeStatus {
            did: "offline".into(),
            peers: 0,
            uptime_secs: 0,
            version: env!("CARGO_PKG_VERSION").into(),
            modules: vec![
                ModuleStatus {
                    name: "Identity".into(),
                    status: "offline".into(),
                },
                ModuleStatus {
                    name: "Messaging".into(),
                    status: "offline".into(),
                },
                ModuleStatus {
                    name: "Governance".into(),
                    status: "offline".into(),
                },
                ModuleStatus {
                    name: "Social".into(),
                    status: "offline".into(),
                },
                ModuleStatus {
                    name: "Payments".into(),
                    status: "offline".into(),
                },
                ModuleStatus {
                    name: "Storage".into(),
                    status: "offline".into(),
                },
                ModuleStatus {
                    name: "AI".into(),
                    status: "offline".into(),
                },
                ModuleStatus {
                    name: "Browser".into(),
                    status: "offline".into(),
                },
            ],
            api_connected: false,
        }),
    }
}

#[tauri::command]
async fn get_wallet_balances(api: State<'_, Arc<ApiClient>>) -> Result<Vec<WalletBalance>, String> {
    let did = api.identity_did.read().await;
    if let Some(ref did) = *did
        && let Ok(wallet) = api
            .get::<WalletResponse>(&format!("/wallets/{}", did))
            .await
    {
        return Ok(wallet
            .balances
            .into_iter()
            .map(|b| WalletBalance {
                token: b.token,
                balance: b.amount,
                usd_value: None,
            })
            .collect());
    }

    Ok(vec![
        WalletBalance {
            token: "ETH".into(),
            balance: "0.000".into(),
            usd_value: Some("$0.00".into()),
        },
        WalletBalance {
            token: "NOUS".into(),
            balance: "0.000".into(),
            usd_value: None,
        },
        WalletBalance {
            token: "USDC".into(),
            balance: "0.000".into(),
            usd_value: Some("$0.00".into()),
        },
    ])
}

#[tauri::command]
async fn get_identity(api: State<'_, Arc<ApiClient>>) -> Result<IdentityInfo, String> {
    let did = api.identity_did.read().await;
    if let Some(ref did) = *did
        && let Ok(info) = api
            .get::<IdentityInfo>(&format!("/identities/{}", did))
            .await
    {
        return Ok(info);
    }

    // Create a new identity
    #[derive(Serialize)]
    struct CreateReq {
        display_name: Option<String>,
    }

    match api
        .post::<IdentityInfo>(
            "/identities",
            &CreateReq {
                display_name: Some("Nous Desktop".into()),
            },
        )
        .await
    {
        Ok(info) => {
            *api.identity_did.write().await = Some(info.did.clone());
            Ok(info)
        }
        Err(_) => Ok(IdentityInfo {
            did: "offline — start API server".into(),
            display_name: None,
            signing_key_type: "ed25519".into(),
            exchange_key_type: "x25519".into(),
        }),
    }
}

#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ── Terminal Types ────────────────────────────────────────────────────────

type TerminalSessions = Arc<Mutex<HashMap<String, Terminal>>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenCell {
    pub ch: String,
    pub fg: SerColor,
    pub bg: SerColor,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub inverse: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum SerColor {
    Default,
    Rgb { r: u8, g: u8, b: u8 },
    Indexed(u8),
}

impl From<TermColor> for SerColor {
    fn from(c: TermColor) -> Self {
        match c {
            TermColor::Default => SerColor::Default,
            TermColor::Rgb(r, g, b) => SerColor::Rgb { r, g, b },
            TermColor::Indexed(idx) => SerColor::Indexed(idx),
        }
    }
}

fn render_row_to_cells(row: &RenderRow) -> Vec<ScreenCell> {
    row.cells
        .iter()
        .map(|cell: &TermCell| ScreenCell {
            ch: cell.ch.to_string(),
            fg: cell.style.fg.into(),
            bg: cell.style.bg.into(),
            bold: cell.style.bold,
            italic: cell.style.italic,
            underline: cell.style.underline,
            strikethrough: cell.style.strikethrough,
            inverse: cell.style.inverse,
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenState {
    pub rows: Vec<Vec<ScreenCell>>,
    pub cursor_row: u16,
    pub cursor_col: u16,
    pub title: String,
}

// ── Terminal Commands ─────────────────────────────────────────────────────

#[tauri::command]
fn terminal_spawn(
    sessions: State<'_, TerminalSessions>,
    rows: Option<u16>,
    cols: Option<u16>,
    shell: Option<String>,
) -> Result<String, String> {
    let config = TerminalConfig {
        rows: rows.unwrap_or(24),
        cols: cols.unwrap_or(80),
        shell: shell.unwrap_or_else(|| std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())),
        ..Default::default()
    };

    let terminal = Terminal::spawn(config).map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();

    sessions
        .lock()
        .map_err(|e| format!("lock poisoned: {e}"))?
        .insert(id.clone(), terminal);

    Ok(id)
}

#[tauri::command]
fn terminal_write(
    sessions: State<'_, TerminalSessions>,
    id: String,
    data: String,
) -> Result<(), String> {
    let mut map = sessions.lock().map_err(|e| format!("lock poisoned: {e}"))?;
    let terminal = map
        .get_mut(&id)
        .ok_or_else(|| format!("no session: {id}"))?;
    terminal.write(data.as_bytes()).map_err(|e| e.to_string())
}

#[tauri::command]
fn terminal_read(sessions: State<'_, TerminalSessions>, id: String) -> Result<Vec<u8>, String> {
    let mut map = sessions.lock().map_err(|e| format!("lock poisoned: {e}"))?;
    let terminal = map
        .get_mut(&id)
        .ok_or_else(|| format!("no session: {id}"))?;

    // Read from PTY and process through VT parser
    let _ = terminal.tick().map_err(|e| e.to_string())?;
    let data = terminal.try_read().map_err(|e| e.to_string())?;
    if !data.is_empty() {
        terminal.process(&data);
    }
    Ok(data)
}

#[tauri::command]
fn terminal_resize(
    sessions: State<'_, TerminalSessions>,
    id: String,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    let mut map = sessions.lock().map_err(|e| format!("lock poisoned: {e}"))?;
    let terminal = map
        .get_mut(&id)
        .ok_or_else(|| format!("no session: {id}"))?;
    terminal.resize(rows, cols).map_err(|e| e.to_string())
}

#[tauri::command]
fn terminal_close(sessions: State<'_, TerminalSessions>, id: String) -> Result<(), String> {
    let mut map = sessions.lock().map_err(|e| format!("lock poisoned: {e}"))?;
    map.remove(&id).ok_or_else(|| format!("no session: {id}"))?;
    Ok(())
}

#[tauri::command]
fn terminal_screen(
    sessions: State<'_, TerminalSessions>,
    id: String,
) -> Result<ScreenState, String> {
    let mut map = sessions.lock().map_err(|e| format!("lock poisoned: {e}"))?;
    let terminal = map
        .get_mut(&id)
        .ok_or_else(|| format!("no session: {id}"))?;

    // Drain any pending PTY output into the VT parser before reading screen
    let _ = terminal.tick();

    let screen_rows = terminal.screen();
    let (cursor_row, cursor_col) = terminal.cursor_position();
    let title = terminal.title().to_string();

    Ok(ScreenState {
        rows: screen_rows.iter().map(render_row_to_cells).collect(),
        cursor_row,
        cursor_col,
        title,
    })
}

// ── System Tray ───────────────────────────────────────────────────────────

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

// ── Run ───────────────────────────────────────────────────────────────────

pub fn run() {
    let api = Arc::new(ApiClient::new());
    let terminal_sessions: TerminalSessions = Arc::new(Mutex::new(HashMap::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .manage(api)
        .manage(terminal_sessions)
        .invoke_handler(tauri::generate_handler![
            get_node_status,
            get_wallet_balances,
            get_identity,
            app_version,
            terminal_spawn,
            terminal_write,
            terminal_read,
            terminal_resize,
            terminal_close,
            terminal_screen,
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
    fn api_client_creates() {
        let client = ApiClient::new();
        assert_eq!(client.base_url, DEFAULT_API_URL);
    }

    #[test]
    fn version_not_empty() {
        let v = app_version();
        assert!(!v.is_empty());
    }

    #[test]
    fn node_status_serializes() {
        let status = NodeStatus {
            did: "did:key:zTest".into(),
            peers: 3,
            uptime_secs: 120,
            version: "0.1.0".into(),
            modules: vec![ModuleStatus {
                name: "Identity".into(),
                status: "active".into(),
            }],
            api_connected: true,
        };
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: NodeStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.did, "did:key:zTest");
        assert!(deserialized.api_connected);
    }

    #[test]
    fn wallet_balance_serializes() {
        let balance = WalletBalance {
            token: "ETH".into(),
            balance: "1.5".into(),
            usd_value: Some("$3000".into()),
        };
        let json = serde_json::to_string(&balance).unwrap();
        let deserialized: WalletBalance = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.token, "ETH");
    }

    #[test]
    fn identity_info_serializes() {
        let info = IdentityInfo {
            did: "did:key:zTest".into(),
            display_name: Some("Test".into()),
            signing_key_type: "ed25519".into(),
            exchange_key_type: "x25519".into(),
        };
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: IdentityInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.did, "did:key:zTest");
    }
}

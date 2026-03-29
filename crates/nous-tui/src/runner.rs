use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::App;
use crate::client::ApiClient;
use crate::config::TuiConfig;
use crate::poll::{self, PollConfig, apply_event};
use crate::theme::Theme;
use crate::views;
use crate::widgets;

/// Run the Nous TUI application.
///
/// This is the top-level entry point that sets up the terminal, runs the event
/// loop, and restores terminal state on exit.
pub async fn run(config: TuiConfig) -> io::Result<()> {
    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Application state
    let mut app = App::new(config.clone());
    let client = ApiClient::new(&config.api_url);
    app.set_api_client(client.clone());

    // Background poller
    let (mut poll_rx, poll_handle) =
        poll::spawn_poller(client, app.local_did.clone(), PollConfig::default());

    // Main loop
    let result = event_loop(&mut terminal, &mut app, &mut poll_rx).await;

    // Cleanup
    poll_handle.abort();
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    poll_rx: &mut tokio::sync::mpsc::Receiver<poll::PollEvent>,
) -> io::Result<()> {
    loop {
        // Draw
        terminal.draw(|f| {
            let area = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(area);

            // Background
            let bg = ratatui::widgets::Block::default().style(Theme::base());
            f.render_widget(bg, area);

            widgets::render_header(f, chunks[0], &app.tabs);
            views::render_tab(f, chunks[1], app);
            widgets::render_status_bar(f, chunks[2], app.peer_count, &app.local_did);
        })?;

        if !app.running {
            break;
        }

        // Handle events with 50ms poll interval for smooth rendering
        tokio::select! {
            // Terminal input events
            _ = tokio::time::sleep(Duration::from_millis(50)) => {
                while event::poll(Duration::ZERO)? {
                    if let Event::Key(key) = event::read()? {
                        handle_key(app, key);
                    }
                }
            }
            // Background API poll events
            Some(event) = poll_rx.recv() => {
                apply_event(app, event);
            }
        }
    }

    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.quit();
        }
        KeyCode::Esc => {
            app.quit();
        }
        KeyCode::Tab => {
            app.tabs.next();
        }
        KeyCode::BackTab => {
            app.tabs.prev();
        }
        KeyCode::Up => match app.tabs.active {
            crate::tabs::Tab::Marketplace => app.marketplace_select_up(),
            crate::tabs::Tab::Browser => app.browser_select_up(),
            _ => app.scroll_up(),
        },
        KeyCode::Down => match app.tabs.active {
            crate::tabs::Tab::Marketplace => app.marketplace_select_down(),
            crate::tabs::Tab::Browser => app.browser_select_down(),
            _ => {
                let max = app.messages.len().saturating_sub(1);
                app.scroll_down(max);
            }
        },
        KeyCode::Enter => {
            app.submit_input();
        }
        KeyCode::Backspace => {
            app.input.backspace();
        }
        KeyCode::Delete => {
            app.input.delete();
        }
        KeyCode::Left => {
            if app.tabs.active == crate::tabs::Tab::Marketplace {
                app.marketplace_toggle_tab();
            } else {
                app.input.move_left();
            }
        }
        KeyCode::Right => {
            if app.tabs.active == crate::tabs::Tab::Marketplace {
                app.marketplace_toggle_tab();
            } else {
                app.input.move_right();
            }
        }
        KeyCode::Home => {
            app.input.home();
        }
        KeyCode::End => {
            app.input.end();
        }
        KeyCode::Char(c) => {
            app.handle_key(c);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        App::new(TuiConfig::default())
    }

    #[test]
    fn handle_ctrl_q_quits() {
        let mut app = test_app();
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
        );
        assert!(!app.running);
    }

    #[test]
    fn handle_esc_quits() {
        let mut app = test_app();
        handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.running);
    }

    #[test]
    fn handle_tab_cycles() {
        let mut app = test_app();
        handle_key(&mut app, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.tabs.active, crate::tabs::Tab::Messages);
    }

    #[test]
    fn handle_backtab_cycles_back() {
        let mut app = test_app();
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT),
        );
        assert_eq!(app.tabs.active, crate::tabs::Tab::Settings);
    }

    #[test]
    fn handle_char_input() {
        let mut app = test_app();
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
        );
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
        );
        assert_eq!(app.input.value, "hi");
    }

    #[test]
    fn handle_number_switches_tab() {
        let mut app = test_app();
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE),
        );
        assert_eq!(app.tabs.active, crate::tabs::Tab::Wallet);
    }

    #[test]
    fn handle_backspace() {
        let mut app = test_app();
        app.input.insert('a');
        app.input.insert('b');
        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
        );
        assert_eq!(app.input.value, "a");
    }

    #[test]
    fn handle_enter_submits() {
        let mut app = test_app();
        app.input.insert('h');
        app.input.insert('i');
        handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.input.is_empty());
    }

    #[test]
    fn handle_arrow_keys() {
        let mut app = test_app();
        // Up/Down shouldn't crash on empty messages
        handle_key(&mut app, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        handle_key(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn up_down_navigates_marketplace() {
        let mut app = test_app();
        app.tabs.select(crate::tabs::Tab::Marketplace);
        app.listings.push(crate::client::ListingItem {
            id: "l1".into(),
            seller_did: "d".into(),
            title: "A".into(),
            description: "".into(),
            category: "".into(),
            price_token: "".into(),
            price_amount: "".into(),
            status: "".into(),
            created_at: "".into(),
            tags: vec![],
        });
        app.listings.push(crate::client::ListingItem {
            id: "l2".into(),
            seller_did: "d".into(),
            title: "B".into(),
            description: "".into(),
            category: "".into(),
            price_token: "".into(),
            price_amount: "".into(),
            status: "".into(),
            created_at: "".into(),
            tags: vec![],
        });

        handle_key(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.marketplace_selected, 1);
        handle_key(&mut app, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.marketplace_selected, 0);
    }

    #[test]
    fn left_right_toggles_marketplace_tab() {
        let mut app = test_app();
        app.tabs.select(crate::tabs::Tab::Marketplace);
        assert_eq!(app.marketplace_tab, crate::app::MarketplaceSubTab::Listings);
        handle_key(&mut app, KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(app.marketplace_tab, crate::app::MarketplaceSubTab::Orders);
        handle_key(&mut app, KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.marketplace_tab, crate::app::MarketplaceSubTab::Listings);
    }

    #[test]
    fn handle_home_end() {
        let mut app = test_app();
        app.input.insert('a');
        app.input.insert('b');
        handle_key(&mut app, KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
        assert_eq!(app.input.cursor, 0);
        handle_key(&mut app, KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
        assert_eq!(app.input.cursor, 2);
    }
}

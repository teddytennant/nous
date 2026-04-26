//! Tab views — every view follows the same editorial spine:
//!   1. A 2-line eyebrow header (`title` + meta-caps subtitle in stone)
//!   2. A hairline rule
//!   3. The content body, padded
//!   4. A meta-caps key footer
//!
//! Lists use a single oxblood `▌` margin bar to mark selection rather than a
//! washed background fill.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::app::App;
use crate::tabs::Tab;
use crate::theme::Theme;
use crate::widgets::{render_inset_block, selection_mark};

/// Render the active tab's content into the given area.
pub fn render_tab(f: &mut Frame, area: Rect, app: &App) {
    // Layout: eyebrow (2) | rule (1) | body (min) | rule (1) | footer (1)
    if area.height < 5 {
        // Too small for full editorial layout — render body only.
        dispatch_body(f, area, app);
        return;
    }
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    render_eyebrow(f, layout[0], app.tabs.active);

    // Hairline under the eyebrow.
    let rule = Block::default()
        .borders(Borders::TOP)
        .border_style(Theme::rule());
    f.render_widget(rule, layout[1]);

    dispatch_body(f, layout[2], app);

    // Hairline above the footer.
    let rule_b = Block::default()
        .borders(Borders::TOP)
        .border_style(Theme::rule());
    f.render_widget(rule_b, layout[3]);

    render_keys_footer(f, layout[4], app.tabs.active);
}

fn dispatch_body(f: &mut Frame, area: Rect, app: &App) {
    match app.tabs.active {
        Tab::Feed => render_feed(f, area, app),
        Tab::Messages => render_messages(f, area, app),
        Tab::Governance => render_governance(f, area, app),
        Tab::Wallet => render_wallet(f, area, app),
        Tab::Marketplace => render_marketplace(f, area, app),
        Tab::Browser => render_browser(f, area, app),
        Tab::Identity => render_identity(f, area, app),
        Tab::Peers => render_peers(f, area, app),
        Tab::Settings => render_settings(f, area, app),
    }
}

fn render_eyebrow(f: &mut Frame, area: Rect, tab: Tab) {
    if area.height == 0 {
        return;
    }
    // 2-cell left gutter.
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(area);

    let title = Span::styled(tab.label_caps(), Theme::title());
    let subtitle = Span::styled(format!("  {}", tab.subtitle()), Theme::meta());
    let line = Line::from(vec![title, subtitle]);
    let p = Paragraph::new(line);
    f.render_widget(p, cols[1]);
}

fn render_keys_footer(f: &mut Frame, area: Rect, tab: Tab) {
    if area.height == 0 {
        return;
    }
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(area);

    let mut spans: Vec<Span<'_>> = Vec::new();
    for (i, (key, label)) in tab.keys().iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  \u{00b7}  ", Theme::meta()));
        }
        spans.push(Span::styled(*key, Theme::accent()));
        spans.push(Span::styled(" ", Theme::meta()));
        spans.push(Span::styled(label.to_uppercase(), Theme::meta()));
    }
    spans.push(Span::styled("  \u{00b7}  ", Theme::meta()));
    spans.push(Span::styled("TAB", Theme::accent()));
    spans.push(Span::styled(" SWITCH", Theme::meta()));
    spans.push(Span::styled("  \u{00b7}  ", Theme::meta()));
    spans.push(Span::styled("?", Theme::accent()));
    spans.push(Span::styled(" KEYS", Theme::meta()));

    let p = Paragraph::new(Line::from(spans));
    f.render_widget(p, cols[1]);
}

// ── individual tab bodies ──────────────────────────────────────────────────

fn render_feed(f: &mut Frame, area: Rect, app: &App) {
    if app.feed_items.is_empty() {
        return render_empty(f, area, "no posts yet");
    }

    let items: Vec<ListItem> = app
        .feed_items
        .iter()
        .map(|item| {
            let header = Line::from(vec![
                Span::styled("  ", Theme::base()),
                Span::styled(&item.author, Theme::bold()),
                Span::styled("   ", Theme::base()),
                Span::styled(&item.timestamp, Theme::meta()),
            ]);
            let content = Line::from(vec![
                Span::styled("  ", Theme::base()),
                Span::styled(&item.content, Theme::base()),
            ]);
            let meta = Line::from(vec![
                Span::styled("  ", Theme::base()),
                Span::styled(format!("{} reactions", item.reactions), Theme::meta()),
                Span::styled("   ", Theme::base()),
                Span::styled(format!("{} replies", item.replies), Theme::meta()),
            ]);
            let blank = Line::from("");
            ListItem::new(vec![header, content, meta, blank])
        })
        .collect();

    let list = List::new(items).block(render_inset_block());
    f.render_widget(list, area);
}

fn render_messages(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    if app.messages.is_empty() {
        render_empty(f, chunks[0], "no messages");
    } else {
        let items: Vec<ListItem> = app
            .visible_messages()
            .iter()
            .map(|msg| {
                let header = Line::from(vec![
                    Span::styled("  ", Theme::base()),
                    Span::styled(&msg.sender, Theme::bold()),
                    Span::styled("   ", Theme::base()),
                    Span::styled(&msg.timestamp, Theme::meta()),
                ]);
                let content = Line::from(vec![
                    Span::styled("  ", Theme::base()),
                    Span::styled(&msg.content, Theme::base()),
                ]);
                ListItem::new(vec![header, content, Line::from("")])
            })
            .collect();

        let list = List::new(items).block(render_inset_block());
        f.render_widget(list, chunks[0]);
    }

    // Hairline above the input row.
    let rule = Block::default()
        .borders(Borders::TOP)
        .border_style(Theme::rule());
    f.render_widget(rule, chunks[1]);

    // Input row — caret in oxblood, then input value.
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(chunks[2]);
    let input_text = app.input.display_value();
    let line = if app.input.is_empty() {
        Line::from(vec![
            Span::styled("\u{203a} ", Theme::accent()),
            Span::styled("type a message", Theme::meta()),
        ])
    } else {
        Line::from(vec![
            Span::styled("\u{203a} ", Theme::accent()),
            Span::styled(input_text, Theme::base()),
        ])
    };
    f.render_widget(Paragraph::new(line), cols[1]);
}

fn render_governance(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Length(1),
            Constraint::Percentage(50),
        ])
        .split(area);

    if app.daos.is_empty() {
        render_empty(f, chunks[0], "no DAOs");
    } else {
        let items: Vec<ListItem> = app
            .daos
            .iter()
            .map(|dao| {
                let header = Line::from(vec![
                    Span::styled("  ", Theme::base()),
                    Span::styled(&dao.name, Theme::bold()),
                    Span::styled("   ", Theme::base()),
                    Span::styled(format!("{} members", dao.member_count), Theme::meta()),
                ]);
                let desc = Line::from(vec![
                    Span::styled("  ", Theme::base()),
                    Span::styled(&dao.description, Theme::base()),
                ]);
                ListItem::new(vec![header, desc, Line::from("")])
            })
            .collect();
        let list = List::new(items).block(render_inset_block());
        f.render_widget(list, chunks[0]);
    }

    let rule = Block::default()
        .borders(Borders::TOP)
        .border_style(Theme::rule());
    f.render_widget(rule, chunks[1]);

    if app.proposals.is_empty() {
        render_empty(f, chunks[2], "no proposals");
    } else {
        let items: Vec<ListItem> = app
            .proposals
            .iter()
            .map(|prop| {
                let status_style = match prop.status.as_str() {
                    "Active" => Theme::success(),
                    "Rejected" => Theme::error(),
                    _ => Theme::meta(),
                };
                let header = Line::from(vec![
                    Span::styled("  ", Theme::base()),
                    Span::styled(&prop.title, Theme::bold()),
                    Span::styled("   ", Theme::base()),
                    Span::styled(prop.status.to_uppercase(), status_style),
                ]);
                let desc = Line::from(vec![
                    Span::styled("  ", Theme::base()),
                    Span::styled(&prop.description, Theme::meta()),
                ]);
                ListItem::new(vec![header, desc, Line::from("")])
            })
            .collect();
        let list = List::new(items).block(render_inset_block());
        f.render_widget(list, chunks[2]);
    }
}

fn render_wallet(f: &mut Frame, area: Rect, app: &App) {
    if app.balances.is_empty() {
        return render_empty(f, area, "no wallet connected");
    }

    let items: Vec<ListItem> = app
        .balances
        .iter()
        .map(|b| {
            let line = Line::from(vec![
                Span::styled("  ", Theme::base()),
                Span::styled(&b.token, Theme::meta()),
                Span::styled("   ", Theme::base()),
                Span::styled(&b.amount, Theme::base()),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(render_inset_block());
    f.render_widget(list, area);
}

fn render_marketplace(f: &mut Frame, area: Rect, app: &App) {
    use crate::app::MarketplaceSubTab;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(area);

    // Sub-tab strip — meta caps, ivory bold on active.
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(chunks[0]);
    let active_listings = app.marketplace_tab == MarketplaceSubTab::Listings;
    let line = Line::from(vec![
        Span::styled(
            "LISTINGS",
            if active_listings {
                Theme::title()
            } else {
                Theme::meta()
            },
        ),
        Span::styled("    ", Theme::base()),
        Span::styled(
            "ORDERS",
            if !active_listings {
                Theme::title()
            } else {
                Theme::meta()
            },
        ),
        Span::styled("      ", Theme::base()),
        Span::styled(format!("{} listings", app.listings.len()), Theme::meta()),
        Span::styled("   ", Theme::base()),
        Span::styled(format!("{} orders", app.orders.len()), Theme::meta()),
    ]);
    f.render_widget(Paragraph::new(line), cols[1]);

    // Hairline.
    let rule = Block::default()
        .borders(Borders::TOP)
        .border_style(Theme::rule());
    f.render_widget(rule, chunks[1]);

    match app.marketplace_tab {
        MarketplaceSubTab::Listings => render_marketplace_listings(f, chunks[2], app),
        MarketplaceSubTab::Orders => render_marketplace_orders(f, chunks[2], app),
    }
}

fn render_marketplace_listings(f: &mut Frame, area: Rect, app: &App) {
    if app.listings.is_empty() {
        return render_empty(f, area, "no listings available");
    }

    let items: Vec<ListItem> = app
        .listings
        .iter()
        .enumerate()
        .map(|(i, listing)| {
            let is_selected = i == app.marketplace_selected;
            let mark = selection_mark(is_selected);
            let header = Line::from(vec![
                mark[0].clone(),
                mark[1].clone(),
                Span::styled(&listing.title, Theme::bold()),
                Span::styled("   ", Theme::base()),
                Span::styled(
                    format!("{} {}", listing.price_amount, listing.price_token),
                    Theme::accent(),
                ),
            ]);
            let meta = Line::from(vec![
                Span::styled("  ", Theme::base()),
                Span::styled(&listing.category, Theme::meta()),
                Span::styled("   ", Theme::base()),
                Span::styled(
                    listing.status.to_uppercase(),
                    match listing.status.as_str() {
                        "Active" => Theme::success(),
                        "Sold" => Theme::meta(),
                        _ => Theme::error(),
                    },
                ),
                Span::styled("   ", Theme::base()),
                Span::styled(listing.tags.join(", "), Theme::meta()),
            ]);
            ListItem::new(vec![header, meta, Line::from("")])
        })
        .collect();

    let list = List::new(items).block(render_inset_block());
    f.render_widget(list, area);
}

fn render_marketplace_orders(f: &mut Frame, area: Rect, app: &App) {
    if app.orders.is_empty() {
        return render_empty(f, area, "no orders");
    }

    let items: Vec<ListItem> = app
        .orders
        .iter()
        .enumerate()
        .map(|(i, order)| {
            let is_selected = i == app.marketplace_selected;
            let mark = selection_mark(is_selected);
            let status_style = match order.status.as_str() {
                "Completed" => Theme::success(),
                "Cancelled" | "Refunded" | "Disputed" => Theme::error(),
                _ => Theme::meta(),
            };
            let header = Line::from(vec![
                mark[0].clone(),
                mark[1].clone(),
                Span::styled(&order.id, Theme::bold()),
                Span::styled("   ", Theme::base()),
                Span::styled(order.status.to_uppercase(), status_style),
            ]);
            let detail = Line::from(vec![
                Span::styled("  ", Theme::base()),
                Span::styled(format!("{} {}", order.amount, order.token), Theme::base()),
                Span::styled("   ", Theme::base()),
                Span::styled(&order.created_at, Theme::meta()),
            ]);
            ListItem::new(vec![header, detail, Line::from("")])
        })
        .collect();

    let list = List::new(items).block(render_inset_block());
    f.render_widget(list, area);
}

fn render_browser(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(chunks[0]);

    let stats = vec![
        Line::from(vec![
            Span::styled("OPEN TABS  ", Theme::meta()),
            Span::styled(app.browser_urls.len().to_string(), Theme::base()),
        ]),
        Line::from(vec![
            Span::styled("HISTORY    ", Theme::meta()),
            Span::styled(
                format!("{} entries", app.browser_history_count),
                Theme::base(),
            ),
        ]),
        Line::from(vec![
            Span::styled("BLOCKED    ", Theme::meta()),
            Span::styled(
                format!("{} requests", app.browser_blocked_count),
                Theme::success(),
            ),
            Span::styled("   ", Theme::base()),
            Span::styled(format!("{} rules", app.browser_filter_rules), Theme::meta()),
        ]),
    ];
    let stats_widget = Paragraph::new(stats).wrap(Wrap { trim: false });
    f.render_widget(stats_widget, cols[1]);

    let rule = Block::default()
        .borders(Borders::TOP)
        .border_style(Theme::rule());
    f.render_widget(rule, chunks[1]);

    if app.browser_urls.is_empty() {
        return render_empty(f, chunks[2], "no tabs open");
    }

    let items: Vec<ListItem> = app
        .browser_urls
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            let is_selected = i == app.browser_selected;
            let mark = selection_mark(is_selected);
            let pin = if tab.pinned {
                Span::styled("PIN ", Theme::accent())
            } else {
                Span::styled("    ", Theme::base())
            };
            let header = Line::from(vec![
                mark[0].clone(),
                mark[1].clone(),
                pin,
                Span::styled(&tab.title, Theme::bold()),
            ]);
            let url_line = Line::from(vec![
                Span::styled("      ", Theme::base()),
                Span::styled(&tab.url, Theme::meta()),
                Span::styled("   ", Theme::base()),
                Span::styled(
                    tab.status.to_uppercase(),
                    match tab.status.as_str() {
                        "Ready" => Theme::success(),
                        "Loading" => Theme::accent(),
                        _ => Theme::error(),
                    },
                ),
            ]);
            ListItem::new(vec![header, url_line, Line::from("")])
        })
        .collect();

    let list = List::new(items).block(render_inset_block());
    f.render_widget(list, chunks[2]);
}

fn render_identity(f: &mut Frame, area: Rect, app: &App) {
    let did_display = if app.local_did.is_empty() {
        "no identity".to_string()
    } else {
        app.local_did.clone()
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("DID     ", Theme::meta()),
            Span::styled(&did_display, Theme::base()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("STATUS  ", Theme::meta()),
            if app.connected {
                Span::styled("CONNECTED", Theme::success())
            } else {
                Span::styled("DISCONNECTED", Theme::error())
            },
        ]),
    ];

    let p = Paragraph::new(lines)
        .block(render_inset_block())
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn render_peers(f: &mut Frame, area: Rect, app: &App) {
    let lines = vec![
        Line::from(vec![
            Span::styled("PEERS    ", Theme::meta()),
            Span::styled(app.peer_count.to_string(), Theme::base()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("NODE     ", Theme::meta()),
            Span::styled(
                app.node_status
                    .as_deref()
                    .unwrap_or("unknown")
                    .to_uppercase(),
                if app.connected {
                    Theme::success()
                } else {
                    Theme::meta()
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("VERSION  ", Theme::meta()),
            Span::styled(
                app.node_version.as_deref().unwrap_or("\u{2014}"),
                Theme::base(),
            ),
        ]),
        Line::from(vec![
            Span::styled("UPTIME   ", Theme::meta()),
            Span::styled(format_uptime(app.node_uptime), Theme::base()),
        ]),
    ];

    let p = Paragraph::new(lines)
        .block(render_inset_block())
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn render_settings(f: &mut Frame, area: Rect, app: &App) {
    let lines = vec![
        Line::from(vec![
            Span::styled("THEME         ", Theme::meta()),
            Span::styled(&app.config.theme, Theme::base()),
        ]),
        Line::from(vec![
            Span::styled("API URL       ", Theme::meta()),
            Span::styled(&app.config.api_url, Theme::base()),
        ]),
        Line::from(vec![
            Span::styled("TIMESTAMPS    ", Theme::meta()),
            Span::styled(
                if app.config.show_timestamps {
                    "ON"
                } else {
                    "OFF"
                },
                Theme::base(),
            ),
        ]),
        Line::from(vec![
            Span::styled("MAX MESSAGES  ", Theme::meta()),
            Span::styled(app.config.max_visible_messages.to_string(), Theme::base()),
        ]),
    ];

    let p = Paragraph::new(lines)
        .block(render_inset_block())
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn render_empty(f: &mut Frame, area: Rect, msg: &str) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(area);
    let line = Line::from(Span::styled(msg, Theme::meta()));
    f.render_widget(Paragraph::new(line), cols[1]);
}

fn format_uptime(ms: Option<u64>) -> String {
    match ms {
        None => "\u{2014}".to_string(),
        Some(ms) => {
            let secs = ms / 1000;
            let mins = secs / 60;
            let hours = mins / 60;
            if hours > 0 {
                format!("{}h {}m", hours, mins % 60)
            } else if mins > 0 {
                format!("{}m {}s", mins, secs % 60)
            } else {
                format!("{}s", secs)
            }
        }
    }
}

/// Render a centered help modal listing all global keybindings.
pub fn render_help_modal(f: &mut Frame, area: Rect) {
    use crate::widgets::{centered_rect, render_modal_block};

    let modal = centered_rect(60, 70, area);
    // Clear underneath by rendering a filled raised-surface block.
    let bg = Block::default().style(ratatui::style::Style::default().bg(Theme::BG_RAISED));
    f.render_widget(bg, modal);

    let block = render_modal_block("keybindings");
    let inner = block.inner(modal);
    f.render_widget(block, modal);

    let rows: &[(&str, &str)] = &[
        ("?", "toggle this overlay"),
        ("ESC / Ctrl-Q", "quit"),
        ("TAB / SHIFT-TAB", "next / previous tab"),
        ("1\u{2026}9", "jump to tab"),
        ("\u{2191} \u{2193}", "scroll / select"),
        ("\u{2190} \u{2192}", "switch sub-tab / move cursor"),
        ("ENTER", "send / activate"),
        ("BACKSPACE / DEL", "edit input"),
        ("HOME / END", "input start / end"),
    ];

    let mut lines: Vec<Line<'_>> = Vec::with_capacity(rows.len() + 2);
    lines.push(Line::from(Span::styled("global", Theme::meta())));
    lines.push(Line::from(""));
    for (key, label) in rows {
        lines.push(Line::from(vec![
            Span::styled(format!("{:<18}", key), Theme::accent()),
            Span::styled(*label, Theme::base()),
        ]));
    }

    let p = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(p, inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{BrowserTabEntry, DisplayMessage, FeedItem, MarketplaceSubTab};
    use crate::client::{BalanceEntry, DaoItem, ListingItem, OrderItem, ProposalItem};
    use crate::config::TuiConfig;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn test_app() -> App {
        App::new(TuiConfig::default())
    }

    fn render_test<F: FnOnce(&mut Frame, Rect, &App)>(app: &App, render_fn: F) {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                render_fn(f, area, app);
            })
            .unwrap();
    }

    #[test]
    fn render_empty_feed() {
        let app = test_app();
        render_test(&app, render_feed);
    }

    #[test]
    fn render_feed_with_items() {
        let mut app = test_app();
        app.add_feed_item(FeedItem {
            author: "did:key:z123".into(),
            content: "Hello world".into(),
            timestamp: "12:00".into(),
            reactions: 5,
            replies: 2,
        });
        app.add_feed_item(FeedItem {
            author: "did:key:z456".into(),
            content: "Second post".into(),
            timestamp: "12:05".into(),
            reactions: 0,
            replies: 0,
        });
        render_test(&app, render_feed);
    }

    #[test]
    fn render_empty_messages() {
        let app = test_app();
        render_test(&app, render_messages);
    }

    #[test]
    fn render_messages_with_content() {
        let mut app = test_app();
        app.add_message(DisplayMessage {
            sender: "alice".into(),
            content: "hey".into(),
            timestamp: "12:00".into(),
        });
        render_test(&app, render_messages);
    }

    #[test]
    fn render_empty_governance() {
        let app = test_app();
        render_test(&app, render_governance);
    }

    #[test]
    fn render_governance_with_data() {
        let mut app = test_app();
        app.daos.push(DaoItem {
            id: "d1".into(),
            name: "Nous DAO".into(),
            description: "Core governance".into(),
            founder_did: "did:key:z123".into(),
            member_count: 42,
            created_at: "2026-03-29".into(),
        });
        app.proposals.push(ProposalItem {
            id: "p1".into(),
            dao_id: "d1".into(),
            title: "Fund development".into(),
            description: "Allocate resources".into(),
            proposer_did: "did:key:z123".into(),
            status: "Active".into(),
            created_at: "2026-03-29".into(),
        });
        render_test(&app, render_governance);
    }

    #[test]
    fn render_empty_wallet() {
        let app = test_app();
        render_test(&app, render_wallet);
    }

    #[test]
    fn render_wallet_with_balances() {
        let mut app = test_app();
        app.balances.push(BalanceEntry {
            token: "ETH".into(),
            amount: "1.5".into(),
        });
        app.balances.push(BalanceEntry {
            token: "NOUS".into(),
            amount: "10000".into(),
        });
        render_test(&app, render_wallet);
    }

    #[test]
    fn render_identity_no_did() {
        let app = test_app();
        render_test(&app, render_identity);
    }

    #[test]
    fn render_identity_connected() {
        let mut app = test_app();
        app.local_did = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".into();
        app.connected = true;
        render_test(&app, render_identity);
    }

    #[test]
    fn render_peers_disconnected() {
        let app = test_app();
        render_test(&app, render_peers);
    }

    #[test]
    fn render_peers_connected() {
        let mut app = test_app();
        app.peer_count = 7;
        app.connected = true;
        app.node_status = Some("running".into());
        app.node_version = Some("0.1.0".into());
        app.node_uptime = Some(3661000);
        render_test(&app, render_peers);
    }

    #[test]
    fn render_settings_default() {
        let app = test_app();
        render_test(&app, render_settings);
    }

    #[test]
    fn format_uptime_none() {
        assert_eq!(format_uptime(None), "\u{2014}");
    }

    #[test]
    fn format_uptime_seconds() {
        assert_eq!(format_uptime(Some(45000)), "45s");
    }

    #[test]
    fn format_uptime_minutes() {
        assert_eq!(format_uptime(Some(125000)), "2m 5s");
    }

    #[test]
    fn format_uptime_hours() {
        assert_eq!(format_uptime(Some(3661000)), "1h 1m");
    }

    #[test]
    fn render_empty_marketplace() {
        let app = test_app();
        render_test(&app, render_marketplace);
    }

    #[test]
    fn render_marketplace_with_listings() {
        let mut app = test_app();
        app.listings.push(ListingItem {
            id: "listing:abc".into(),
            seller_did: "did:key:z123".into(),
            title: "Vintage Keyboard".into(),
            description: "Mechanical, cherry blues".into(),
            category: "Physical".into(),
            price_token: "ETH".into(),
            price_amount: "0.5".into(),
            status: "Active".into(),
            created_at: "2026-03-29".into(),
            tags: vec!["electronics".into(), "vintage".into()],
        });
        render_test(&app, render_marketplace);
    }

    #[test]
    fn render_marketplace_orders_tab() {
        let mut app = test_app();
        app.marketplace_tab = MarketplaceSubTab::Orders;
        app.orders.push(OrderItem {
            id: "order:xyz".into(),
            listing_id: "listing:abc".into(),
            buyer_did: "did:key:buyer".into(),
            seller_did: "did:key:seller".into(),
            token: "ETH".into(),
            amount: "0.5".into(),
            status: "Shipped".into(),
            created_at: "2026-03-29".into(),
        });
        render_test(&app, render_marketplace);
    }

    #[test]
    fn render_empty_browser() {
        let app = test_app();
        render_test(&app, render_browser);
    }

    #[test]
    fn render_browser_with_tabs() {
        let mut app = test_app();
        app.browser_urls.push(BrowserTabEntry {
            title: "Nous Docs".into(),
            url: "https://nous.dev/docs".into(),
            status: "Ready".into(),
            pinned: false,
        });
        app.browser_urls.push(BrowserTabEntry {
            title: "IPFS Gateway".into(),
            url: "ipfs://QmTest123".into(),
            status: "Loading".into(),
            pinned: true,
        });
        app.browser_history_count = 42;
        app.browser_blocked_count = 1337;
        app.browser_filter_rules = 15;
        render_test(&app, render_browser);
    }

    #[test]
    fn render_tab_dispatches_correctly() {
        let mut app = test_app();
        for tab in Tab::all() {
            app.tabs.select(*tab);
            render_test(&app, render_tab);
        }
    }

    #[test]
    fn render_help_modal_smoke() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                render_help_modal(f, area);
            })
            .unwrap();
    }
}

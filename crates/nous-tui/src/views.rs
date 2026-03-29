use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph, Wrap};

use crate::app::App;
use crate::tabs::Tab;
use crate::theme::Theme;
use crate::widgets::render_content_block;

/// Render the active tab's content into the given area.
pub fn render_tab(f: &mut Frame, area: Rect, app: &App) {
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

fn render_feed(f: &mut Frame, area: Rect, app: &App) {
    if app.feed_items.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled("No posts yet", Theme::dim())))
            .block(render_content_block("Feed"));
        f.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .feed_items
        .iter()
        .map(|item| {
            let header = Line::from(vec![
                Span::styled(&item.author, Theme::accent()),
                Span::styled("  ", Theme::base()),
                Span::styled(&item.timestamp, Theme::dim()),
            ]);
            let content = Line::from(Span::styled(&item.content, Theme::base()));
            let meta = Line::from(vec![
                Span::styled(format!("{} reactions", item.reactions), Theme::dim()),
                Span::styled("  ", Theme::base()),
                Span::styled(format!("{} replies", item.replies), Theme::dim()),
            ]);
            let blank = Line::from("");
            ListItem::new(vec![header, content, meta, blank])
        })
        .collect();

    let list = List::new(items).block(render_content_block("Feed"));
    f.render_widget(list, area);
}

fn render_messages(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(area);

    // Message list
    if app.messages.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled("No messages", Theme::dim())))
            .block(render_content_block("Messages"));
        f.render_widget(empty, chunks[0]);
    } else {
        let items: Vec<ListItem> = app
            .visible_messages()
            .iter()
            .map(|msg| {
                let header = Line::from(vec![
                    Span::styled(&msg.sender, Theme::accent()),
                    Span::styled("  ", Theme::base()),
                    Span::styled(&msg.timestamp, Theme::dim()),
                ]);
                let content = Line::from(Span::styled(&msg.content, Theme::base()));
                ListItem::new(vec![header, content])
            })
            .collect();

        let list = List::new(items).block(render_content_block("Messages"));
        f.render_widget(list, chunks[0]);
    }

    // Input bar
    let input_text = app.input.display_value();
    let input_style = if app.input.is_empty() {
        Theme::dim()
    } else {
        Theme::input()
    };
    let input = Paragraph::new(Line::from(Span::styled(input_text, input_style)))
        .block(render_content_block(""));
    f.render_widget(input, chunks[1]);
}

fn render_governance(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // DAOs
    if app.daos.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled("No DAOs", Theme::dim())))
            .block(render_content_block("DAOs"));
        f.render_widget(empty, chunks[0]);
    } else {
        let items: Vec<ListItem> = app
            .daos
            .iter()
            .map(|dao| {
                let header = Line::from(vec![
                    Span::styled(&dao.name, Theme::accent()),
                    Span::styled(format!("  {} members", dao.member_count), Theme::dim()),
                ]);
                let desc = Line::from(Span::styled(&dao.description, Theme::base()));
                ListItem::new(vec![header, desc])
            })
            .collect();
        let list = List::new(items).block(render_content_block("DAOs"));
        f.render_widget(list, chunks[0]);
    }

    // Proposals
    if app.proposals.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled("No proposals", Theme::dim())))
            .block(render_content_block("Proposals"));
        f.render_widget(empty, chunks[1]);
    } else {
        let items: Vec<ListItem> = app
            .proposals
            .iter()
            .map(|prop| {
                let status_style = match prop.status.as_str() {
                    "Active" => Theme::success(),
                    "Rejected" => Theme::error(),
                    _ => Theme::dim(),
                };
                let header = Line::from(vec![
                    Span::styled(&prop.title, Theme::bold()),
                    Span::styled("  ", Theme::base()),
                    Span::styled(&prop.status, status_style),
                ]);
                let desc = Line::from(Span::styled(&prop.description, Theme::dim()));
                ListItem::new(vec![header, desc])
            })
            .collect();
        let list = List::new(items).block(render_content_block("Proposals"));
        f.render_widget(list, chunks[1]);
    }
}

fn render_wallet(f: &mut Frame, area: Rect, app: &App) {
    if app.balances.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No wallet connected",
            Theme::dim(),
        )))
        .block(render_content_block("Wallet"));
        f.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .balances
        .iter()
        .map(|b| {
            let line = Line::from(vec![
                Span::styled(&b.token, Theme::bold()),
                Span::styled("  ", Theme::base()),
                Span::styled(&b.amount, Theme::accent()),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(render_content_block("Balances"));
    f.render_widget(list, area);
}

fn render_marketplace(f: &mut Frame, area: Rect, app: &App) {
    use crate::app::MarketplaceSubTab;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    // Sub-tab bar
    let tabs_line = Line::from(vec![
        Span::styled(
            " Listings ",
            if app.marketplace_tab == MarketplaceSubTab::Listings {
                Theme::accent()
            } else {
                Theme::dim()
            },
        ),
        Span::styled("  ", Theme::base()),
        Span::styled(
            " Orders ",
            if app.marketplace_tab == MarketplaceSubTab::Orders {
                Theme::accent()
            } else {
                Theme::dim()
            },
        ),
        Span::styled("  ", Theme::base()),
        Span::styled(format!("{} listings", app.listings.len()), Theme::dim()),
        Span::styled("  ", Theme::base()),
        Span::styled(format!("{} orders", app.orders.len()), Theme::dim()),
    ]);
    let tab_bar = Paragraph::new(tabs_line).block(render_content_block("Marketplace"));
    f.render_widget(tab_bar, chunks[0]);

    match app.marketplace_tab {
        MarketplaceSubTab::Listings => render_marketplace_listings(f, chunks[1], app),
        MarketplaceSubTab::Orders => render_marketplace_orders(f, chunks[1], app),
    }
}

fn render_marketplace_listings(f: &mut Frame, area: Rect, app: &App) {
    if app.listings.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No listings available",
            Theme::dim(),
        )))
        .block(render_content_block("Listings"));
        f.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .listings
        .iter()
        .enumerate()
        .map(|(i, listing)| {
            let is_selected = i == app.marketplace_selected;
            let title_style = if is_selected {
                Theme::selected()
            } else {
                Theme::bold()
            };
            let header = Line::from(vec![
                Span::styled(
                    if is_selected { "\u{25b6} " } else { "  " },
                    Theme::accent(),
                ),
                Span::styled(&listing.title, title_style),
                Span::styled("  ", Theme::base()),
                Span::styled(
                    format!("{} {}", listing.price_amount, listing.price_token),
                    Theme::accent(),
                ),
            ]);
            let meta = Line::from(vec![
                Span::styled("  ", Theme::base()),
                Span::styled(&listing.category, Theme::dim()),
                Span::styled("  ", Theme::base()),
                Span::styled(
                    &listing.status,
                    match listing.status.as_str() {
                        "Active" => Theme::success(),
                        "Sold" => Theme::dim(),
                        _ => Theme::error(),
                    },
                ),
                Span::styled("  ", Theme::base()),
                Span::styled(listing.tags.join(", "), Theme::dim()),
            ]);
            let blank = Line::from("");
            ListItem::new(vec![header, meta, blank])
        })
        .collect();

    let list = List::new(items).block(render_content_block("Listings"));
    f.render_widget(list, area);
}

fn render_marketplace_orders(f: &mut Frame, area: Rect, app: &App) {
    if app.orders.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled("No orders", Theme::dim())))
            .block(render_content_block("Orders"));
        f.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .orders
        .iter()
        .enumerate()
        .map(|(i, order)| {
            let is_selected = i == app.marketplace_selected;
            let status_style = match order.status.as_str() {
                "Completed" => Theme::success(),
                "Cancelled" | "Refunded" => Theme::error(),
                "Disputed" => Theme::error(),
                _ => Theme::accent(),
            };
            let header = Line::from(vec![
                Span::styled(
                    if is_selected { "\u{25b6} " } else { "  " },
                    Theme::accent(),
                ),
                Span::styled(
                    &order.id,
                    if is_selected {
                        Theme::selected()
                    } else {
                        Theme::bold()
                    },
                ),
                Span::styled("  ", Theme::base()),
                Span::styled(&order.status, status_style),
            ]);
            let detail = Line::from(vec![
                Span::styled("  ", Theme::base()),
                Span::styled(format!("{} {}", order.amount, order.token), Theme::accent()),
                Span::styled("  ", Theme::base()),
                Span::styled(&order.created_at, Theme::dim()),
            ]);
            let blank = Line::from("");
            ListItem::new(vec![header, detail, blank])
        })
        .collect();

    let list = List::new(items).block(render_content_block("Orders"));
    f.render_widget(list, area);
}

fn render_browser(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(1)])
        .split(area);

    // Browser stats panel
    let stats = vec![
        Line::from(vec![
            Span::styled("Open tabs  ", Theme::dim()),
            Span::styled(app.browser_urls.len().to_string(), Theme::accent()),
        ]),
        Line::from(vec![
            Span::styled("History  ", Theme::dim()),
            Span::styled(
                format!("{} entries", app.browser_history_count),
                Theme::base(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Blocked  ", Theme::dim()),
            Span::styled(
                format!("{} requests", app.browser_blocked_count),
                Theme::success(),
            ),
            Span::styled("  ", Theme::base()),
            Span::styled(format!("{} rules", app.browser_filter_rules), Theme::dim()),
        ]),
    ];
    let stats_widget = Paragraph::new(stats)
        .block(render_content_block("Browser"))
        .wrap(Wrap { trim: false });
    f.render_widget(stats_widget, chunks[0]);

    // Open tabs list
    if app.browser_urls.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled("No tabs open", Theme::dim())))
            .block(render_content_block("Tabs"));
        f.render_widget(empty, chunks[1]);
    } else {
        let items: Vec<ListItem> = app
            .browser_urls
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let is_selected = i == app.browser_selected;
                let pin_marker = if tab.pinned { "[pin] " } else { "" };
                let header = Line::from(vec![
                    Span::styled(
                        if is_selected { "\u{25b6} " } else { "  " },
                        Theme::accent(),
                    ),
                    Span::styled(pin_marker, Theme::dim()),
                    Span::styled(
                        &tab.title,
                        if is_selected {
                            Theme::selected()
                        } else {
                            Theme::bold()
                        },
                    ),
                ]);
                let url_line = Line::from(vec![
                    Span::styled("  ", Theme::base()),
                    Span::styled(&tab.url, Theme::dim()),
                    Span::styled("  ", Theme::base()),
                    Span::styled(
                        &tab.status,
                        match tab.status.as_str() {
                            "Ready" => Theme::success(),
                            "Loading" => Theme::accent(),
                            _ => Theme::error(),
                        },
                    ),
                ]);
                let blank = Line::from("");
                ListItem::new(vec![header, url_line, blank])
            })
            .collect();

        let list = List::new(items).block(render_content_block("Tabs"));
        f.render_widget(list, chunks[1]);
    }
}

fn render_identity(f: &mut Frame, area: Rect, app: &App) {
    let did_display = if app.local_did.is_empty() {
        "No identity".to_string()
    } else {
        app.local_did.clone()
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("DID  ", Theme::dim()),
            Span::styled(&did_display, Theme::accent()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Status  ", Theme::dim()),
            if app.connected {
                Span::styled("Connected", Theme::success())
            } else {
                Span::styled("Disconnected", Theme::error())
            },
        ]),
    ];

    let content = Paragraph::new(lines)
        .block(render_content_block("Identity"))
        .wrap(Wrap { trim: false });
    f.render_widget(content, area);
}

fn render_peers(f: &mut Frame, area: Rect, app: &App) {
    let lines = vec![
        Line::from(vec![
            Span::styled("Connected peers  ", Theme::dim()),
            Span::styled(app.peer_count.to_string(), Theme::accent()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Node  ", Theme::dim()),
            Span::styled(
                app.node_status.as_deref().unwrap_or("unknown"),
                if app.connected {
                    Theme::success()
                } else {
                    Theme::dim()
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("Version  ", Theme::dim()),
            Span::styled(app.node_version.as_deref().unwrap_or("-"), Theme::base()),
        ]),
        Line::from(vec![
            Span::styled("Uptime  ", Theme::dim()),
            Span::styled(format_uptime(app.node_uptime), Theme::base()),
        ]),
    ];

    let content = Paragraph::new(lines)
        .block(render_content_block("Network"))
        .wrap(Wrap { trim: false });
    f.render_widget(content, area);
}

fn render_settings(f: &mut Frame, area: Rect, app: &App) {
    let lines = vec![
        Line::from(vec![
            Span::styled("Theme  ", Theme::dim()),
            Span::styled(&app.config.theme, Theme::base()),
        ]),
        Line::from(vec![
            Span::styled("API URL  ", Theme::dim()),
            Span::styled(&app.config.api_url, Theme::base()),
        ]),
        Line::from(vec![
            Span::styled("Timestamps  ", Theme::dim()),
            Span::styled(
                if app.config.show_timestamps {
                    "on"
                } else {
                    "off"
                },
                Theme::base(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Max messages  ", Theme::dim()),
            Span::styled(app.config.max_visible_messages.to_string(), Theme::base()),
        ]),
    ];

    let content = Paragraph::new(lines)
        .block(render_content_block("Settings"))
        .wrap(Wrap { trim: false });
    f.render_widget(content, area);
}

fn format_uptime(ms: Option<u64>) -> String {
    match ms {
        None => "-".to_string(),
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
        assert_eq!(format_uptime(None), "-");
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
        // Test each tab renders without panic
        for tab in Tab::all() {
            app.tabs.select(*tab);
            render_test(&app, render_tab);
        }
    }
}

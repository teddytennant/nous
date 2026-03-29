use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::tabs::{Tab, TabState};
use crate::theme::Theme;

pub fn render_header(f: &mut Frame, area: Rect, tab_state: &TabState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(30)])
        .split(area);

    // Logo
    let logo = Paragraph::new(Line::from(vec![
        Span::styled("N", Theme::accent()),
        Span::styled("OUS", Theme::bold()),
    ]))
    .alignment(Alignment::Left);
    f.render_widget(logo, chunks[0]);

    // Tabs
    let tab_spans: Vec<Span> = Tab::all()
        .iter()
        .flat_map(|tab| {
            let style = if *tab == tab_state.active {
                Theme::accent()
            } else {
                Theme::dim()
            };
            vec![
                Span::styled(tab.label(), style),
                Span::styled("  ", Theme::base()),
            ]
        })
        .collect();

    let tabs = Paragraph::new(Line::from(tab_spans)).alignment(Alignment::Right);
    f.render_widget(tabs, chunks[1]);
}

pub fn render_status_bar(f: &mut Frame, area: Rect, peer_count: usize, did: &str) {
    let status = Line::from(vec![
        Span::styled(" Peers: ", Theme::dim()),
        Span::styled(peer_count.to_string(), Theme::accent()),
        Span::styled("  |  ", Theme::dim()),
        Span::styled("DID: ", Theme::dim()),
        Span::styled(truncate_did(did), Theme::base()),
    ]);

    let bar = Paragraph::new(status)
        .style(Theme::status_bar())
        .alignment(Alignment::Left);

    f.render_widget(bar, area);
}

pub fn render_content_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .title(Span::styled(format!(" {title} "), Theme::title()))
}

fn truncate_did(did: &str) -> String {
    if did.len() > 24 {
        format!("{}...{}", &did[..12], &did[did.len() - 8..])
    } else {
        did.to_string()
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_did() {
        let did = "did:key:z123";
        assert_eq!(truncate_did(did), did);
    }

    #[test]
    fn truncate_long_did() {
        let did = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
        let truncated = truncate_did(did);
        assert!(truncated.contains("..."));
        assert!(truncated.len() < did.len());
    }

    #[test]
    fn centered_rect_is_within_bounds() {
        let area = Rect::new(0, 0, 100, 50);
        let centered = centered_rect(50, 50, area);
        assert!(centered.x >= area.x);
        assert!(centered.y >= area.y);
        assert!(centered.right() <= area.right());
        assert!(centered.bottom() <= area.bottom());
    }

    #[test]
    fn content_block_has_borders() {
        let block = render_content_block("Test");
        // Just verify it doesn't panic
        let _ = block;
    }
}

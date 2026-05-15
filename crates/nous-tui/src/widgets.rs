//! Editorial chrome — wordmark header, status bar, content frame.
//!
//! Hairlines, not borders. The header is two lines: wordmark + truncated DID,
//! then a 1px rule. The status bar is meta-caps separated by `·` with a rule
//! above. Content panels never get a full `Borders::ALL` box; they get a
//! single top hairline (and a trailing rule when the design calls for it).
//! Full borders are reserved for modal overlays.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};

use crate::tabs::{Tab, TabState};
use crate::theme::Theme;

/// Render the top-of-page wordmark + identity row, terminated by a hairline.
///
/// `area` should be at least 2 cells tall: line 1 is content, line 2 is the
/// rule. If the caller only allocates 1 cell we render the content and skip
/// the rule rather than panicking.
pub fn render_header(f: &mut Frame, area: Rect, tab_state: &TabState, did: &str) {
    let chunks = if area.height >= 2 {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1)])
            .split(area)
    };

    // Inner left/right split for the wordmark + meta line.
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(2), // gutter
            Constraint::Min(20),   // wordmark
            Constraint::Min(20),   // did meta (right-aligned)
            Constraint::Length(2), // gutter
        ])
        .split(chunks[0]);

    // Wordmark: lowercase "nous" — type carries the brand, not the accent.
    let wordmark =
        Paragraph::new(Line::from(Span::styled("nous", Theme::title()))).alignment(Alignment::Left);
    f.render_widget(wordmark, cols[1]);

    // DID in mono (terminals are mono by default), stone, right-aligned.
    let did_meta = Paragraph::new(Line::from(Span::styled(truncate_did(did), Theme::meta())))
        .alignment(Alignment::Right);
    f.render_widget(did_meta, cols[2]);

    // Hairline rule across the full width.
    if chunks.len() == 2 {
        let rule = Block::default()
            .borders(Borders::TOP)
            .border_style(Theme::rule());
        f.render_widget(rule, chunks[1]);
    }

    // Tabs render under the rule, left to the renderer of the tab strip.
    let _ = tab_state; // currently unused — tab strip is rendered separately.
}

/// Status bar — meta-caps `PEERS · 12  ·  REACH · ON  ·  ?` separated by
/// thin middle dots. The bar carries a hairline above it.
pub fn render_status_bar(
    f: &mut Frame,
    area: Rect,
    peer_count: usize,
    reachability: Option<&str>,
    hint: Option<&str>,
) {
    if area.height == 0 {
        return;
    }
    let chunks = if area.height >= 2 {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1)])
            .split(area)
    };

    if chunks.len() == 2 {
        let rule = Block::default()
            .borders(Borders::TOP)
            .border_style(Theme::rule());
        f.render_widget(rule, chunks[0]);
    }

    let bar_area = chunks.last().copied().unwrap_or(area);
    let inner = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(bar_area);

    let sep = || Span::styled("  \u{00b7}  ", Theme::meta());

    let mut spans: Vec<Span<'_>> = Vec::new();
    spans.push(Span::styled("PEERS", Theme::meta()));
    spans.push(Span::styled(" ", Theme::meta()));
    spans.push(Span::styled(peer_count.to_string(), Theme::base()));
    spans.push(sep());
    spans.push(Span::styled("REACH", Theme::meta()));
    spans.push(Span::styled(" ", Theme::meta()));
    spans.push(Span::styled(
        reachability.unwrap_or("\u{2014}").to_string(),
        Theme::base(),
    ));

    if let Some(h) = hint {
        spans.push(sep());
        // Hints are oxblood — the only place the accent appears in the bar.
        spans.push(Span::styled(h.to_string(), Theme::accent()));
    } else {
        spans.push(sep());
        spans.push(Span::styled("?", Theme::accent()));
        spans.push(Span::styled(" KEYS", Theme::meta()));
    }

    let bar = Paragraph::new(Line::from(spans))
        .style(Theme::status_bar())
        .alignment(Alignment::Left);

    f.render_widget(bar, inner[1]);
}

/// A content "block" with a single top hairline rule and 2-cell horizontal,
/// 1-cell vertical padding. Reserve full borders for modals only.
pub fn render_content_block(_title: &str) -> Block<'static> {
    Block::default()
        .borders(Borders::TOP)
        .border_style(Theme::rule())
        .padding(Padding::new(2, 2, 1, 1))
}

/// A bare content surface with no rule — just editorial breathing space.
/// Use this for the inner content area when an outer rule is already drawn.
pub fn render_inset_block() -> Block<'static> {
    Block::default()
        .borders(Borders::NONE)
        .padding(Padding::new(2, 2, 1, 1))
}

/// Modal overlay block — the only place a full `Borders::ALL` is allowed.
/// Rendered with the rule color and a meta-caps title.
pub fn render_modal_block(title: &str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::rule())
        .style(ratatui::style::Style::default().bg(Theme::BG_RAISED))
        .padding(Padding::new(2, 2, 1, 1))
        .title(Span::styled(
            format!(" {} ", title.to_uppercase()),
            Theme::meta(),
        ))
}

/// Truncated DID, e.g. `did:key:z6Mk…J9`. Geist Mono is implicit in a terminal.
pub fn truncate_did(did: &str) -> String {
    if did.is_empty() {
        return "\u{2014}".to_string();
    }
    if did.len() > 24 {
        format!("{}\u{2026}{}", &did[..12], &did[did.len() - 8..])
    } else {
        did.to_string()
    }
}

/// Center a rect within an outer area, used for modal overlays.
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

/// Selection mark — a single oxblood `▌` glyph + space, intended as the
/// leftmost spans of a selected row. Returns the two spans the row should
/// prepend; unselected rows should prepend two stone-padding spans of the
/// same width so column alignment is preserved.
pub fn selection_mark(selected: bool) -> [Span<'static>; 2] {
    if selected {
        [
            Span::styled("\u{258c}", Theme::accent()),
            Span::styled(" ", Theme::base()),
        ]
    } else {
        [
            Span::styled(" ", Theme::base()),
            Span::styled(" ", Theme::base()),
        ]
    }
}

/// Tab strip — meta-caps labels with an oxblood underbar on the active tab.
/// `area` should be at least 2 cells tall (label row + underbar row).
pub fn render_tab_strip(f: &mut Frame, area: Rect, tab_state: &TabState) {
    if area.height == 0 {
        return;
    }
    let row_layout = if area.height >= 2 {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1)])
            .split(area)
    };

    // 2-cell left gutter.
    let gutter_h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(row_layout[0]);

    // Build labels and remember their starting columns to draw the underbar.
    let mut spans: Vec<Span<'_>> = Vec::new();
    let mut bar_segments: Vec<(u16, u16)> = Vec::new(); // (start_col_within_inner, width)
    let mut col: u16 = 0;
    let active = tab_state.active;
    for (i, t) in Tab::all().iter().enumerate() {
        let label = t.label_caps();
        let style = if *t == active {
            Theme::title()
        } else {
            Theme::meta()
        };
        let span = Span::styled(label, style);
        let w = label.chars().count() as u16;
        spans.push(span);
        if *t == active {
            bar_segments.push((col, w));
        }
        col = col.saturating_add(w);
        if i + 1 < Tab::all().len() {
            spans.push(Span::styled("  ", Theme::base()));
            col = col.saturating_add(2);
        }
    }

    let labels = Paragraph::new(Line::from(spans));
    f.render_widget(labels, gutter_h[1]);

    if row_layout.len() == 2 {
        // Background hairline beneath the strip (rule across full width).
        let rule = Block::default()
            .borders(Borders::TOP)
            .border_style(Theme::rule());
        f.render_widget(rule, row_layout[1]);

        // Oxblood underbar over the active tab — overdraws the rule.
        let inner = gutter_h[1];
        for (start, width) in bar_segments {
            let x = inner.x.saturating_add(start);
            let bar_area = Rect {
                x,
                y: row_layout[1].y,
                width: width.min(inner.width.saturating_sub(start)),
                height: 1,
            };
            let bar_str: String = std::iter::repeat('\u{2581}')
                .take(bar_area.width as usize)
                .collect();
            let bar = Paragraph::new(Line::from(Span::styled(bar_str, Theme::accent())));
            f.render_widget(bar, bar_area);
        }
    }
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
        assert!(truncated.contains("\u{2026}"));
        assert!(truncated.len() < did.len());
    }

    #[test]
    fn truncate_empty_did_emdash() {
        assert_eq!(truncate_did(""), "\u{2014}");
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
    fn content_block_uses_top_rule_only() {
        let _ = render_content_block("Test");
        // Method exists and doesn't panic; ratatui doesn't expose Borders
        // on Block directly, so this is a smoke test.
    }

    #[test]
    fn modal_block_uses_full_borders() {
        let _ = render_modal_block("Help");
    }

    #[test]
    fn selection_mark_distinguishes_state() {
        let on = selection_mark(true);
        let off = selection_mark(false);
        assert_ne!(on[0].content, off[0].content);
        assert_eq!(on[0].content, "\u{258c}");
    }
}

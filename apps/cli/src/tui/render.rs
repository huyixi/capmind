use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use super::chat_widget::ChatWidget;
use super::types::FocusArea;

pub fn draw(frame: &mut Frame<'_>, widget: &mut ChatWidget) {
    let area = frame.area();
    let layout =
        Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);

    render_history(frame, layout[0], widget);
    render_composer(frame, layout[1], widget);
}

fn render_history(frame: &mut Frame<'_>, area: ratatui::layout::Rect, widget: &ChatWidget) {
    let title = match widget.status_line() {
        Some(status) if !status.is_empty() => format!("History ({status})"),
        _ => "History".to_string(),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(match widget.focus() {
            FocusArea::History => Style::default().fg(Color::Cyan),
            FocusArea::Composer => Style::default(),
        });

    if widget.history().is_empty() {
        frame.render_widget(
            Paragraph::new("No history yet.")
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
        return;
    }

    let items: Vec<ListItem<'_>> = widget
        .history()
        .iter()
        .map(|cell| {
            let ts = cell.created_at.format("%H:%M:%S");
            let line = format!(
                "{ts} [{}] {} - {}",
                cell.kind_label(),
                cell.title,
                cell.body_preview
            );
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    let mut state = ListState::default();
    state.select(widget.selected_history());
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_composer(frame: &mut Frame<'_>, area: ratatui::layout::Rect, widget: &mut ChatWidget) {
    let focused = widget.focus() == FocusArea::Composer;
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Composer")
        .border_style(if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        });
    let inner = block.inner(area);

    let composer = widget.bottom_pane_mut().composer_mut();
    composer.ensure_cursor_visible(inner.height);
    let text = composer.lines().join("\n");
    let paragraph = Paragraph::new(text)
        .block(block)
        .scroll((composer.scroll_y(), 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);

    if focused {
        let row = composer
            .cursor_row()
            .saturating_sub(composer.scroll_y() as usize) as u16;
        if row < inner.height {
            let x = inner.x.saturating_add(composer.cursor_col() as u16);
            let y = inner.y.saturating_add(row);
            frame.set_cursor_position((x, y));
        }
    }
}

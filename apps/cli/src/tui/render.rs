use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph, Wrap};

use super::chat_widget::ChatWidget;
use super::theme::{PaneColors, UiTheme};
use super::types::FocusArea;

const COMPOSER_MIN_INPUT_ROWS: u16 = 5;
const COMPOSER_H_INSET: u16 = 0;
const COMPOSER_OUTER_BOTTOM_MARGIN: u16 = 1;
const COMPOSER_INNER_PADDING_X: u16 = 0;
const COMPOSER_INNER_PADDING_TOP: u16 = 1;
const COMPOSER_INNER_PADDING_BOTTOM: u16 = 1;

pub fn draw(frame: &mut Frame<'_>, widget: &mut ChatWidget, theme: &UiTheme) {
    let area = frame.area();
    if area.width == 0 || area.height == 0 {
        return;
    }

    let (composer_lines, composer_char_count) = {
        let composer = widget.bottom_pane_mut().composer_mut();
        let lines = composer.lines();
        let line_count = lines.len() as u16;
        let char_count: usize = lines.iter().map(|line| line.chars().count()).sum();
        (line_count, char_count)
    };
    let layout = compute_layout(area, composer_lines);

    render_history(frame, layout.history, widget);
    render_composer(
        frame,
        layout.composer,
        layout.composer_input,
        widget,
        &theme.composer,
    );
    render_composer_counter(frame, area, composer_char_count);
}

fn render_history(frame: &mut Frame<'_>, area: Rect, widget: &ChatWidget) {
    if area.height == 0 {
        return;
    }

    let focused = widget.focus() == FocusArea::History;
    let pane_style = Style::default().fg(Color::Reset);
    let header_style = Style::default()
        .fg(Color::Reset)
        .add_modifier(if focused {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });

    frame.render_widget(Block::default().style(pane_style), area);
    let layout = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area);

    let title = match widget.status_line() {
        Some(status) if !status.is_empty() => format!("History ({status})"),
        _ => "History".to_string(),
    };
    frame.render_widget(Paragraph::new(title).style(header_style), layout[0]);

    let content_area = layout[1];
    if content_area.height == 0 {
        return;
    }

    if widget.history().is_empty() {
        frame.render_widget(
            Paragraph::new("No history yet.")
                .style(pane_style)
                .wrap(Wrap { trim: false }),
            content_area,
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
        .style(pane_style)
        .highlight_style(
            Style::default()
                .fg(Color::Reset)
                .bg(Color::Reset)
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    let mut state = ListState::default();
    state.select(widget.selected_history());
    frame.render_stateful_widget(list, content_area, &mut state);
}

fn render_composer(
    frame: &mut Frame<'_>,
    area: Rect,
    input_area: Rect,
    widget: &mut ChatWidget,
    colors: &PaneColors,
) {
    if area.height == 0 || area.width == 0 || input_area.height == 0 || input_area.width == 0 {
        return;
    }

    let focused = widget.focus() == FocusArea::Composer;
    let block_style = Style::default().bg(colors.normal_bg);
    let text_style = Style::default().fg(Color::Reset);

    frame.render_widget(Block::default().style(block_style), area);

    let composer = widget.bottom_pane_mut().composer_mut();
    composer.ensure_cursor_visible(input_area.height);
    let text = composer.lines().join("\n");
    let display_text = format!("> {text}");
    let cursor_row = composer.cursor_row().saturating_sub(composer.scroll_y() as usize) as u16;
    let prompt_offset = if cursor_row == 0 { 2 } else { 0 };
    let raw_cursor_col = (composer.cursor_col() as u16).saturating_add(prompt_offset);
    let horizontal_scroll = raw_cursor_col.saturating_sub(input_area.width.saturating_sub(1));
    let paragraph = Paragraph::new(display_text)
        .style(text_style)
        .scroll((composer.scroll_y(), horizontal_scroll));
    frame.render_widget(paragraph, input_area);

    if focused {
        let row = cursor_row;
        if row < input_area.height {
            let max_col = input_area.width.saturating_sub(1);
            let col = raw_cursor_col
                .saturating_sub(horizontal_scroll)
                .min(max_col);
            let x = input_area.x.saturating_add(col);
            let y = input_area.y.saturating_add(row);
            frame.set_cursor_position((x, y));
        }
    }
}

fn render_composer_counter(frame: &mut Frame<'_>, frame_area: Rect, char_count: usize) {
    if frame_area.height == 0 || frame_area.width == 0 {
        return;
    }
    let counter_area = Rect {
        x: frame_area.x,
        y: frame_area.y.saturating_add(frame_area.height.saturating_sub(1)),
        width: frame_area.width,
        height: 1,
    };
    let counter = Paragraph::new(format!("{char_count} chars"))
        .style(Style::default().fg(Color::Reset).add_modifier(Modifier::DIM))
        .alignment(Alignment::Right);
    frame.render_widget(counter, counter_area);
}

#[derive(Debug, Clone, Copy)]
struct FloatingLayout {
    history: Rect,
    composer: Rect,
    composer_input: Rect,
}

fn compute_layout(area: Rect, composer_lines: u16) -> FloatingLayout {
    let bottom_margin = COMPOSER_OUTER_BOTTOM_MARGIN.min(area.height.saturating_sub(1));
    let usable_height = area.height.saturating_sub(bottom_margin);

    let desired_input_rows = composer_lines.max(COMPOSER_MIN_INPUT_ROWS);
    let desired_card_height =
        desired_input_rows + COMPOSER_INNER_PADDING_TOP + COMPOSER_INNER_PADDING_BOTTOM;

    let composer_height = if usable_height <= 3 {
        usable_height
    } else {
        desired_card_height
            .min(usable_height.saturating_sub(1))
            .max(3)
    };

    let history_height = usable_height.saturating_sub(composer_height);

    let inset = COMPOSER_H_INSET.min(area.width.saturating_sub(1) / 2);
    let composer = Rect {
        x: area.x.saturating_add(inset),
        y: area.y.saturating_add(history_height),
        width: area.width.saturating_sub(inset.saturating_mul(2)),
        height: composer_height,
    };
    let input_pad_x = COMPOSER_INNER_PADDING_X.min(composer.width.saturating_sub(1) / 2);
    let input_pad_top = COMPOSER_INNER_PADDING_TOP.min(composer.height.saturating_sub(1));
    let input_pad_bottom = COMPOSER_INNER_PADDING_BOTTOM.min(
        composer
            .height
            .saturating_sub(1)
            .saturating_sub(input_pad_top),
    );
    let composer_input = Rect {
        x: composer.x.saturating_add(input_pad_x),
        y: composer.y.saturating_add(input_pad_top),
        width: composer.width.saturating_sub(input_pad_x.saturating_mul(2)),
        height: composer
            .height
            .saturating_sub(input_pad_top + input_pad_bottom),
    };

    let history = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: history_height,
    };

    FloatingLayout {
        history,
        composer,
        composer_input,
    }
}

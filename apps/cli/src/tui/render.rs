use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph, Wrap};

use super::chat_widget::ChatWidget;
use super::theme::{PaneColors, UiTheme};
use super::types::FocusArea;

const HISTORY_FIXED_HEIGHT: u16 = 3;
const COMPOSER_H_INSET: u16 = 0;
const COMPOSER_INNER_PADDING_X: u16 = 0;
const COMPOSER_INNER_PADDING_TOP: u16 = 1;
const COMPOSER_INNER_PADDING_BOTTOM: u16 = 1;
const COMPOSER_PLACEHOLDER: &str = "What's on your mind?";

pub fn draw(frame: &mut Frame<'_>, widget: &mut ChatWidget, theme: &UiTheme) {
    let area = frame.area();
    if area.width == 0 || area.height == 0 {
        return;
    }

    let layout = compute_layout(area);

    render_history(frame, layout.history, widget, &theme.composer);
    render_composer(frame, layout.composer, layout.composer_input, widget);
}

fn render_history(frame: &mut Frame<'_>, area: Rect, widget: &ChatWidget, colors: &PaneColors) {
    if area.height == 0 {
        return;
    }

    let pane_style = Style::default().fg(Color::Reset).bg(colors.normal_bg);

    frame.render_widget(Block::default().style(pane_style), area);
    let content_area = area;
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
            let ts = cell.created_at.format("%Y-%m-%d %H:%M:%S");
            let text = cell.full_text.replace('\n', " ");
            ListItem::new(format!("{ts} {text}"))
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
        .highlight_symbol("");
    let mut state = ListState::default();
    state.select(widget.selected_history());
    frame.render_stateful_widget(list, content_area, &mut state);
}

fn render_composer(
    frame: &mut Frame<'_>,
    area: Rect,
    input_area: Rect,
    widget: &mut ChatWidget,
) {
    if area.height == 0 || area.width == 0 || input_area.height == 0 || input_area.width == 0 {
        return;
    }

    let focused = widget.focus() == FocusArea::Composer;
    let block_style = Style::default();
    let text_style = Style::default().fg(Color::Reset);

    frame.render_widget(Block::default().style(block_style), area);

    let composer = widget.bottom_pane_mut().composer_mut();
    composer.ensure_cursor_visible(input_area.height);
    let text = composer.lines().join("\n");
    let (display_text, is_placeholder) = composer_display_text(&text);
    let cursor_row = composer.cursor_row().saturating_sub(composer.scroll_y() as usize) as u16;
    let raw_cursor_col = composer.cursor_col() as u16;
    let horizontal_scroll = raw_cursor_col.saturating_sub(input_area.width.saturating_sub(1));
    let paragraph_style = if is_placeholder {
        text_style.add_modifier(Modifier::DIM)
    } else {
        text_style
    };
    let paragraph = Paragraph::new(display_text)
        .style(paragraph_style)
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

fn composer_display_text(text: &str) -> (String, bool) {
    if text.is_empty() {
        (COMPOSER_PLACEHOLDER.to_string(), true)
    } else {
        (text.to_string(), false)
    }
}

#[derive(Debug, Clone, Copy)]
struct FloatingLayout {
    history: Rect,
    composer: Rect,
    composer_input: Rect,
}

fn compute_layout(area: Rect) -> FloatingLayout {
    let usable_height = area.height;
    let history_height = HISTORY_FIXED_HEIGHT.min(usable_height.saturating_sub(1));
    let composer_height = usable_height.saturating_sub(history_height);

    let inset = COMPOSER_H_INSET.min(area.width.saturating_sub(1) / 2);
    let composer = Rect {
        x: area.x.saturating_add(inset),
        y: area.y,
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
        y: area.y.saturating_add(composer_height),
        width: area.width,
        height: history_height,
    };

    FloatingLayout {
        history,
        composer,
        composer_input,
    }
}

#[cfg(test)]
mod tests {
    use super::{composer_display_text, compute_layout};
    use ratatui::layout::Rect;

    #[test]
    fn compute_layout_places_composer_at_top() {
        let area = Rect::new(0, 0, 100, 24);
        let layout = compute_layout(area);
        assert_eq!(layout.composer.y, area.y);
    }

    #[test]
    fn compute_layout_places_history_below_composer() {
        let area = Rect::new(0, 0, 100, 24);
        let layout = compute_layout(area);
        assert_eq!(
            layout.history.y,
            layout.composer.y.saturating_add(layout.composer.height)
        );
    }

    #[test]
    fn compute_layout_keeps_panes_non_overlapping_and_within_bounds() {
        let area = Rect::new(2, 3, 80, 20);
        let layout = compute_layout(area);

        let composer_bottom = layout.composer.y.saturating_add(layout.composer.height);
        assert!(composer_bottom <= layout.history.y);

        let area_bottom = area.y.saturating_add(area.height);
        let history_bottom = layout.history.y.saturating_add(layout.history.height);
        assert!(history_bottom <= area_bottom);
        assert!(layout.composer.x >= area.x);
        assert!(layout.history.x >= area.x);
    }

    #[test]
    fn compute_layout_keeps_history_to_three_rows_when_space_allows() {
        let area = Rect::new(0, 0, 100, 24);
        let layout = compute_layout(area);
        assert_eq!(layout.history.height, 3);
        assert_eq!(layout.composer.height, 21);
    }

    #[test]
    fn composer_display_text_uses_placeholder_when_empty() {
        let (text, is_placeholder) = composer_display_text("");
        assert_eq!(text, "What's on your mind?");
        assert!(is_placeholder);
    }

    #[test]
    fn composer_display_text_uses_input_when_non_empty() {
        let (text, is_placeholder) = composer_display_text("hello");
        assert_eq!(text, "hello");
        assert!(!is_placeholder);
    }
}

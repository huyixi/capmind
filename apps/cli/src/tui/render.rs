use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use unicode_width::UnicodeWidthChar;

use super::chat_widget::ChatWidget;
use super::theme::{PaneColors, UiTheme};
use super::types::FocusArea;

const HISTORY_FIXED_HEIGHT: u16 = 3;
const COMPOSER_H_INSET: u16 = 0;
const COMPOSER_INNER_PADDING_X: u16 = 0;
const COMPOSER_INNER_PADDING_TOP: u16 = 1;
const COMPOSER_INNER_PADDING_BOTTOM: u16 = 1;
const COMPOSER_PLACEHOLDER: &str = "What's on your mind?";
const COMPOSER_EDIT_PLACEHOLDER: &str = "Editing selected memo...";
const COMPOSER_QUIT_CONFIRM_PLACEHOLDER: &str = "Press Esc again to quit";

pub fn draw(frame: &mut Frame<'_>, widget: &mut ChatWidget, theme: &UiTheme) {
    let area = frame.area();
    if area.width == 0 || area.height == 0 {
        return;
    }

    let layout = compute_layout(area);

    render_history(frame, layout.history, widget, &theme.composer);
    render_composer(frame, layout.composer, layout.composer_input, widget);
    render_delete_confirmation(frame, area, widget.delete_confirmation_text());
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

    let history_focused = widget.focus() == FocusArea::History;
    let highlight_style = if history_focused {
        Style::default()
            .fg(Color::Reset)
            .bg(Color::Reset)
            .add_modifier(Modifier::REVERSED)
            .add_modifier(Modifier::BOLD)
    } else {
        pane_style
    };

    let list = List::new(items)
        .style(pane_style)
        .highlight_style(highlight_style)
        .highlight_symbol("");
    let mut state = ListState::default();
    state.select(widget.selected_history());
    frame.render_stateful_widget(list, content_area, &mut state);
}

fn render_composer(frame: &mut Frame<'_>, area: Rect, input_area: Rect, widget: &mut ChatWidget) {
    if area.height == 0 || area.width == 0 || input_area.height == 0 || input_area.width == 0 {
        return;
    }

    let focused = widget.focus() == FocusArea::Composer;
    let block_style = Style::default();
    let text_style = Style::default().fg(Color::Reset);

    frame.render_widget(Block::default().style(block_style), area);

    let is_editing_memo = widget.is_editing_memo();
    let quit_confirmation_pending = widget.quit_confirmation_pending();
    let composer = widget.bottom_pane_mut().composer_mut();
    composer.ensure_cursor_visible(input_area.height);
    let text = composer.lines().join("\n");
    let (display_text, is_placeholder) =
        composer_display_text(&text, is_editing_memo, quit_confirmation_pending);
    let cursor_row_abs = composer.cursor_row();
    let cursor_row = cursor_row_abs.saturating_sub(composer.scroll_y() as usize) as u16;
    let current_line = composer
        .lines()
        .get(cursor_row_abs)
        .map(String::as_str)
        .unwrap_or("");
    let raw_cursor_col = display_width_until_char(current_line, composer.cursor_col());
    let horizontal_scroll = calculate_horizontal_scroll(raw_cursor_col, input_area.width);
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

fn display_width_until_char(input: &str, char_idx: usize) -> u16 {
    let width = input
        .chars()
        .take(char_idx)
        .fold(0usize, |acc, c| acc.saturating_add(UnicodeWidthChar::width(c).unwrap_or(0)));
    width.min(u16::MAX as usize) as u16
}

fn calculate_horizontal_scroll(cursor_col: u16, viewport_width: u16) -> u16 {
    cursor_col.saturating_sub(viewport_width.saturating_sub(1))
}

fn composer_display_text(
    text: &str,
    is_editing_memo: bool,
    quit_confirmation_pending: bool,
) -> (String, bool) {
    if text.is_empty() {
        if quit_confirmation_pending {
            (COMPOSER_QUIT_CONFIRM_PLACEHOLDER.to_string(), true)
        } else if is_editing_memo {
            (COMPOSER_EDIT_PLACEHOLDER.to_string(), true)
        } else {
            (COMPOSER_PLACEHOLDER.to_string(), true)
        }
    } else {
        (text.to_string(), false)
    }
}

fn render_delete_confirmation(frame: &mut Frame<'_>, area: Rect, preview: Option<&str>) {
    let Some(preview) = preview else {
        return;
    };
    if area.width < 24 || area.height < 6 {
        return;
    }

    let popup_width = area.width.min(72);
    let popup_height = 6;
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    let preview_single_line = preview.replace('\n', " ");
    let max_preview_chars = popup_width.saturating_sub(6) as usize;
    let preview_display = truncate_for_popup(&preview_single_line, max_preview_chars);
    let content =
        format!("Delete selected memo?\n\"{preview_display}\"\nEnter/Y/D confirm | Esc/N cancel");

    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Confirm Delete"),
            )
            .style(Style::default().fg(Color::Reset).bg(Color::Reset))
            .wrap(Wrap { trim: true }),
        popup,
    );
}

fn truncate_for_popup(input: &str, limit: usize) -> String {
    if input.chars().count() <= limit {
        return input.to_string();
    }
    let keep = limit.saturating_sub(3);
    let truncated: String = input.chars().take(keep).collect();
    format!("{truncated}...")
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
    use super::{
        calculate_horizontal_scroll, composer_display_text, compute_layout, display_width_until_char,
    };
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
        let (text, is_placeholder) = composer_display_text("", false, false);
        assert_eq!(text, "What's on your mind?");
        assert!(is_placeholder);
    }

    #[test]
    fn composer_display_text_uses_input_when_non_empty() {
        let (text, is_placeholder) = composer_display_text("hello", false, false);
        assert_eq!(text, "hello");
        assert!(!is_placeholder);
    }

    #[test]
    fn composer_display_text_uses_edit_placeholder_in_edit_mode() {
        let (text, is_placeholder) = composer_display_text("", true, false);
        assert_eq!(text, "Editing selected memo...");
        assert!(is_placeholder);
    }

    #[test]
    fn composer_display_text_uses_quit_confirm_placeholder_when_pending() {
        let (text, is_placeholder) = composer_display_text("", false, true);
        assert_eq!(text, "Press Esc again to quit");
        assert!(is_placeholder);
    }

    #[test]
    fn display_width_until_char_counts_cjk_as_double_width() {
        let input = "你好";
        assert_eq!(display_width_until_char(input, input.chars().count()), 4);
    }

    #[test]
    fn display_width_until_char_handles_mixed_ascii_and_cjk() {
        let input = "a你b";
        assert_eq!(display_width_until_char(input, 1), 1);
        assert_eq!(display_width_until_char(input, 2), 3);
        assert_eq!(display_width_until_char(input, 3), 4);
    }

    #[test]
    fn calculate_horizontal_scroll_uses_display_width_cursor() {
        assert_eq!(calculate_horizontal_scroll(4, 3), 2);
        assert_eq!(calculate_horizontal_scroll(2, 3), 0);
    }
}

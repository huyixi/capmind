use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

use super::chat_widget::{ChatWidget, HelpOverlayContext, PageMode};
use super::composer::VimMode;
use super::types::FocusArea;

const HISTORY_FIXED_HEIGHT: u16 = 6;
const COMPOSER_H_INSET: u16 = 0;
const COMPOSER_INNER_PADDING_X: u16 = 0;
const COMPOSER_INNER_PADDING_TOP: u16 = 1;
const COMPOSER_INNER_PADDING_BOTTOM: u16 = 1;
const COMPOSER_PLACEHOLDER: &str = "What's on your mind?";
const COMPOSER_EDIT_PLACEHOLDER: &str = "Editing selected memo...";
const COMPOSER_QUIT_CONFIRM_PLACEHOLDER: &str = "Press Esc again to quit";
const IMAGE_ONLY_MEMO_PLACEHOLDER: &str = "[Image-only memo]";

pub fn draw(frame: &mut Frame<'_>, widget: &mut ChatWidget) {
    let area = frame.area();
    if area.width == 0 || area.height == 0 {
        return;
    }

    match widget.page_mode() {
        PageMode::Composer => render_composer_page(frame, area, widget),
        PageMode::MemoList => render_memo_list_page(frame, area, widget),
    }

    render_help_overlay(frame, area, widget.help_overlay());
    render_delete_confirmation(frame, area, widget.delete_confirmation_text());
}

fn render_composer_page(frame: &mut Frame<'_>, area: Rect, widget: &mut ChatWidget) {
    let layout = if widget.split_list_open() {
        compute_split_layout(area)
    } else {
        compute_composer_only_layout(area)
    };
    let composer_mode = widget.bottom_pane_mut().composer_mut().vim_mode();

    if widget.split_list_open() {
        render_history(frame, layout.history, widget);
    }
    render_composer(
        frame,
        layout.composer,
        layout.composer_input,
        widget,
        composer_mode,
    );
}

fn render_memo_list_page(frame: &mut Frame<'_>, area: Rect, widget: &ChatWidget) {
    let pane_style = Style::default();
    frame.render_widget(Block::default().style(pane_style), area);

    if area.height == 0 {
        return;
    }

    let footer = Rect {
        x: area.x,
        y: area.y.saturating_add(area.height.saturating_sub(1)),
        width: area.width,
        height: 1,
    };
    let list_height = area.height.saturating_sub(1);
    let list_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: list_height,
    };
    let visible_indices = widget.memo_list_visible_indices();

    if list_area.height > 0 {
        if widget.history().is_empty() && widget.memo_list_loading() {
            frame.render_widget(
                Paragraph::new("Fetching memo list...")
                    .style(pane_style)
                    .wrap(Wrap { trim: false }),
                list_area,
            );
        } else if visible_indices.is_empty() {
            frame.render_widget(
                Paragraph::new("No matches.")
                    .style(pane_style)
                    .wrap(Wrap { trim: false }),
                list_area,
            );
        } else {
            let items: Vec<ListItem<'_>> = visible_indices
                .iter()
                .filter_map(|index| widget.history().get(*index))
                .map(|cell| {
                    let memo_text =
                        history_row_display_text(&cell.full_text, cell.memo_id.is_some());
                    ListItem::new(format_memo_list_row(&memo_text, list_area.width as usize))
                })
                .collect();

            let list = List::new(items)
                .style(pane_style)
                .highlight_style(
                    Style::default()
                        .add_modifier(Modifier::REVERSED)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("");
            let mut state = ListState::default();
            state.select(widget.memo_list_selected_visible_index());
            frame.render_stateful_widget(list, list_area, &mut state);
        }
    }

    let selected_memo_time = widget
        .memo_list_selected_cell()
        .map(|cell| cell.created_at.format("%Y-%m-%d %H:%M:%S").to_string());
    let search_query = if widget.memo_list_search_mode() || !widget.memo_list_query().is_empty() {
        Some(widget.memo_list_query())
    } else {
        None
    };
    let footer_text = memo_list_footer_text(
        widget.memo_list_loading(),
        search_query,
        widget.status_message(),
        selected_memo_time.as_deref(),
    );
    frame.render_widget(
        Paragraph::new(footer_text).style(Style::default().add_modifier(Modifier::DIM)),
        footer,
    );
}

fn memo_list_footer_text(
    loading: bool,
    search_query: Option<&str>,
    status_message: Option<&str>,
    selected_memo_time: Option<&str>,
) -> String {
    let mode_label = "[LIST]";
    let detail = if loading {
        Some("Fetching memo list...".to_string())
    } else if let Some(query) = search_query {
        Some(format!("Search: {query}"))
    } else if let Some(status) = status_message {
        Some(status.to_string())
    } else {
        selected_memo_time.map(ToString::to_string)
    };

    if let Some(detail) = detail {
        format!("{mode_label} {detail}")
    } else {
        mode_label.to_string()
    }
}

fn format_memo_list_row(memo: &str, total_width: usize) -> String {
    if total_width == 0 {
        return String::new();
    }
    truncate_with_ellipsis(memo, total_width)
}

fn history_row_display_text(full_text: &str, is_memo_entry: bool) -> String {
    let single_line = full_text.replace('\n', " ");
    if is_memo_entry && single_line.trim().is_empty() {
        return IMAGE_ONLY_MEMO_PLACEHOLDER.to_string();
    }
    single_line
}

fn render_history(frame: &mut Frame<'_>, area: Rect, widget: &ChatWidget) {
    if area.height == 0 {
        return;
    }

    let pane_style = Style::default();

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
            let text = history_row_display_text(&cell.full_text, cell.memo_id.is_some());
            ListItem::new(text)
        })
        .collect();

    let history_focused = widget.focus() == FocusArea::History;
    let highlight_style = if history_focused {
        Style::default()
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

fn render_composer(
    frame: &mut Frame<'_>,
    area: Rect,
    input_area: Rect,
    widget: &mut ChatWidget,
    composer_mode: VimMode,
) {
    if area.height == 0 || area.width == 0 || input_area.height == 0 || input_area.width == 0 {
        return;
    }

    let focused = widget.focus() == FocusArea::Composer;
    let block_style = Style::default();
    let text_style = Style::default();

    frame.render_widget(Block::default().style(block_style), area);

    let status_message = widget.status_message().map(ToString::to_string);
    let quit_confirmation_pending = widget.quit_confirmation_pending();
    let is_editing_memo = widget.is_editing_memo();

    let composer = widget.bottom_pane_mut().composer_mut();
    let text = composer.lines().join("\n");
    let (display_text, is_placeholder) = composer_display_text(&text, is_editing_memo);
    let raw_cursor_col = composer.cursor_display_col().min(u16::MAX as usize) as u16;
    let (cursor_row_abs, cursor_col) = calculate_wrapped_cursor_position(
        composer.lines(),
        composer.cursor_row(),
        raw_cursor_col,
        input_area.width,
    );
    let vertical_scroll = calculate_vertical_scroll(cursor_row_abs, input_area.height);
    let cursor_row = cursor_row_abs.saturating_sub(vertical_scroll);
    let paragraph_style = if is_placeholder {
        text_style.add_modifier(Modifier::DIM)
    } else {
        text_style
    };
    let paragraph = Paragraph::new(display_text)
        .style(paragraph_style)
        .wrap(Wrap { trim: false })
        .scroll((vertical_scroll, 0));
    frame.render_widget(paragraph, input_area);

    let footer = composer_footer_text(
        composer_mode,
        quit_confirmation_pending,
        status_message.as_deref(),
    );
    frame.render_widget(
        Paragraph::new(footer.clone()).style(text_style.add_modifier(Modifier::DIM)),
        Rect {
            x: area.x,
            y: area.y.saturating_add(area.height.saturating_sub(1)),
            width: area.width,
            height: 1,
        },
    );

    if focused {
        let row = cursor_row;
        if row < input_area.height {
            let max_col = input_area.width.saturating_sub(1);
            let col = cursor_col.min(max_col);
            let x = input_area.x.saturating_add(col);
            let y = input_area.y.saturating_add(row);
            frame.set_cursor_position((x, y));
        }
    }
}

fn composer_footer_text(
    mode: VimMode,
    quit_confirmation_pending: bool,
    status_message: Option<&str>,
) -> String {
    let mode_label = if mode == VimMode::Insert {
        "[INSERT]"
    } else {
        "[NORMAL]"
    };

    let suffix = if quit_confirmation_pending {
        Some(COMPOSER_QUIT_CONFIRM_PLACEHOLDER)
    } else {
        status_message
    };

    if let Some(value) = suffix {
        format!("{mode_label} {value}")
    } else {
        mode_label.to_string()
    }
}

fn calculate_wrapped_cursor_position(
    lines: &[String],
    cursor_row: usize,
    cursor_col: u16,
    viewport_width: u16,
) -> (u16, u16) {
    if viewport_width == 0 {
        return (0, 0);
    }

    let width = viewport_width as usize;
    let mut visual_row_abs: u16 = 0;
    let safe_cursor_row = cursor_row.min(lines.len().saturating_sub(1));
    for line in lines.iter().take(safe_cursor_row) {
        let line_width = UnicodeWidthStr::width(line.as_str());
        visual_row_abs = visual_row_abs.saturating_add(wrapped_line_count(line_width, width));
    }

    let cursor_col_usize = cursor_col as usize;
    let wrapped_row_offset = (cursor_col_usize / width).min(u16::MAX as usize) as u16;
    let wrapped_col = (cursor_col_usize % width) as u16;
    (
        visual_row_abs.saturating_add(wrapped_row_offset),
        wrapped_col,
    )
}

fn wrapped_line_count(line_width: usize, viewport_width: usize) -> u16 {
    if viewport_width == 0 {
        return 0;
    }
    if line_width == 0 {
        return 1;
    }
    line_width
        .saturating_sub(1)
        .saturating_div(viewport_width)
        .saturating_add(1)
        .min(u16::MAX as usize) as u16
}

fn calculate_vertical_scroll(cursor_row: u16, viewport_height: u16) -> u16 {
    cursor_row.saturating_sub(viewport_height.saturating_sub(1))
}

fn composer_display_text(text: &str, is_editing_memo: bool) -> (String, bool) {
    if text.is_empty() {
        if is_editing_memo {
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
    let preview_display = truncate_with_ellipsis(&preview_single_line, max_preview_chars);
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
            .style(Style::default())
            .wrap(Wrap { trim: true }),
        popup,
    );
}

fn render_help_overlay(frame: &mut Frame<'_>, area: Rect, context: Option<HelpOverlayContext>) {
    let Some(context) = context else {
        return;
    };
    if area.width < 40 || area.height < 8 {
        return;
    }

    let content = help_overlay_content(context);
    let line_count = content.lines().count().min(u16::MAX as usize) as u16;
    let popup_width = area.width.min(92);
    let popup_height = line_count.saturating_add(2).max(8).min(area.height);
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(content)
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .style(Style::default())
            .wrap(Wrap { trim: true }),
        popup,
    );
}

fn help_overlay_content(context: HelpOverlayContext) -> &'static str {
    match context {
        HelpOverlayContext::ComposerNormal => {
            "Composer NORMAL\n\
:w/:s submit | :W submit+quit on success\n\
:q/:Q quit (confirm if unsaved)\n\
:l open memo list\n\
h/j/k/l move | b 0 $ | x dd edit\n\
i/a/I/A/o/O enter insert actions\n\
? / Esc / q close help"
        }
        HelpOverlayContext::MemoList => {
            "Memo List\n\
j/k or arrows move selection\n\
Ctrl+f/PageDown next page\n\
PageUp previous page\n\
:n next page | :p previous page\n\
/ enter search\n\
Enter apply search | Esc clear search\n\
Enter open selected memo\n\
y copy selected memo\n\
r refresh memo list\n\
d delete selected memo\n\
:c return to composer\n\
:q quit program\n\
? / Esc / q close help"
        }
    }
}

fn truncate_with_ellipsis(input: &str, limit: usize) -> String {
    if limit == 0 {
        return String::new();
    }

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

fn compute_split_layout(area: Rect) -> FloatingLayout {
    let usable_height = area.height;
    let history_height = HISTORY_FIXED_HEIGHT.min(usable_height.saturating_sub(1));
    let composer_height = usable_height.saturating_sub(history_height);

    let inset = COMPOSER_H_INSET;
    let composer = Rect {
        x: area.x.saturating_add(inset),
        y: area.y,
        width: area.width.saturating_sub(inset.saturating_mul(2)),
        height: composer_height,
    };
    let input_pad_x = COMPOSER_INNER_PADDING_X;
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

fn compute_composer_only_layout(area: Rect) -> FloatingLayout {
    let composer = area;
    let input_pad_x = COMPOSER_INNER_PADDING_X;
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

    FloatingLayout {
        history: Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        },
        composer,
        composer_input,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        calculate_vertical_scroll, calculate_wrapped_cursor_position, composer_display_text,
        composer_footer_text, compute_composer_only_layout, compute_split_layout,
        format_memo_list_row, help_overlay_content, history_row_display_text,
        memo_list_footer_text, truncate_with_ellipsis,
    };
    use crate::tui::chat_widget::HelpOverlayContext;
    use crate::tui::composer::VimMode;
    use ratatui::layout::Rect;

    #[test]
    fn compute_layout_places_composer_at_top() {
        let area = Rect::new(0, 0, 100, 24);
        let layout = compute_split_layout(area);
        assert_eq!(layout.composer.y, area.y);
    }

    #[test]
    fn compute_layout_places_history_below_composer() {
        let area = Rect::new(0, 0, 100, 24);
        let layout = compute_split_layout(area);
        assert_eq!(
            layout.history.y,
            layout.composer.y.saturating_add(layout.composer.height)
        );
    }

    #[test]
    fn compute_layout_keeps_panes_non_overlapping_and_within_bounds() {
        let area = Rect::new(2, 3, 80, 20);
        let layout = compute_split_layout(area);

        let composer_bottom = layout.composer.y.saturating_add(layout.composer.height);
        assert!(composer_bottom <= layout.history.y);

        let area_bottom = area.y.saturating_add(area.height);
        let history_bottom = layout.history.y.saturating_add(layout.history.height);
        assert!(history_bottom <= area_bottom);
        assert!(layout.composer.x >= area.x);
        assert!(layout.history.x >= area.x);
    }

    #[test]
    fn compute_layout_keeps_history_to_six_rows_when_space_allows() {
        let area = Rect::new(0, 0, 100, 24);
        let layout = compute_split_layout(area);
        assert_eq!(layout.history.height, 6);
        assert_eq!(layout.composer.height, 18);
    }

    #[test]
    fn compute_composer_only_layout_uses_full_area_for_composer() {
        let area = Rect::new(0, 0, 100, 24);
        let layout = compute_composer_only_layout(area);
        assert_eq!(layout.composer, area);
        assert_eq!(layout.history.height, 0);
    }

    #[test]
    fn composer_display_text_uses_placeholder_when_empty() {
        let (text, is_placeholder) = composer_display_text("", false);
        assert_eq!(text, "What's on your mind?");
        assert!(is_placeholder);
    }

    #[test]
    fn composer_display_text_uses_input_when_non_empty() {
        let (text, is_placeholder) = composer_display_text("hello", false);
        assert_eq!(text, "hello");
        assert!(!is_placeholder);
    }

    #[test]
    fn composer_display_text_uses_edit_placeholder_in_edit_mode() {
        let (text, is_placeholder) = composer_display_text("", true);
        assert_eq!(text, "Editing selected memo...");
        assert!(is_placeholder);
    }

    #[test]
    fn composer_footer_appends_status_after_mode() {
        assert_eq!(
            composer_footer_text(VimMode::Insert, false, Some("saving...")),
            "[INSERT] saving..."
        );
    }

    #[test]
    fn composer_footer_uses_quit_hint_when_pending() {
        assert_eq!(
            composer_footer_text(VimMode::Normal, true, None),
            "[NORMAL] Press Esc again to quit"
        );
    }

    #[test]
    fn composer_footer_uses_mode_when_no_status_or_quit_hint() {
        assert_eq!(
            composer_footer_text(VimMode::Insert, false, None),
            "[INSERT]"
        );
        assert_eq!(
            composer_footer_text(VimMode::Normal, false, None),
            "[NORMAL]"
        );
    }

    #[test]
    fn calculate_wrapped_cursor_position_wraps_long_line() {
        let lines = vec!["abcdef".to_string()];
        assert_eq!(calculate_wrapped_cursor_position(&lines, 0, 5, 4), (1, 1));
        assert_eq!(calculate_wrapped_cursor_position(&lines, 0, 3, 4), (0, 3));
    }

    #[test]
    fn calculate_wrapped_cursor_position_counts_previous_rows() {
        let lines = vec!["abcde".to_string(), "xy".to_string()];
        assert_eq!(calculate_wrapped_cursor_position(&lines, 1, 1, 4), (2, 1));
    }

    #[test]
    fn calculate_vertical_scroll_keeps_cursor_visible() {
        assert_eq!(calculate_vertical_scroll(4, 3), 2);
        assert_eq!(calculate_vertical_scroll(1, 3), 0);
    }

    #[test]
    fn truncate_with_ellipsis_truncates_when_over_limit() {
        assert_eq!(truncate_with_ellipsis("abcdef", 4), "a...");
        assert_eq!(truncate_with_ellipsis("abc", 4), "abc");
    }

    #[test]
    fn format_memo_list_row_uses_memo_text_only_with_ellipsis() {
        let row = format_memo_list_row("abcdefghijklmnopqrstuvwxyz", 10);
        assert_eq!(row, "abcdefg...");
    }

    #[test]
    fn history_row_display_text_uses_placeholder_for_empty_memo_entries() {
        assert_eq!(history_row_display_text("", true), "[Image-only memo]");
        assert_eq!(
            history_row_display_text(" \n\t ", true),
            "[Image-only memo]"
        );
    }

    #[test]
    fn history_row_display_text_keeps_non_memo_empty_entries() {
        assert_eq!(history_row_display_text("", false), "");
    }

    #[test]
    fn help_overlay_content_is_context_specific() {
        let composer = help_overlay_content(HelpOverlayContext::ComposerNormal);
        assert!(composer.contains("Composer NORMAL"));
        assert!(composer.contains(":w/:s submit"));
        assert!(composer.contains(":W submit+quit on success"));

        let memo_list = help_overlay_content(HelpOverlayContext::MemoList);
        assert!(memo_list.contains("Memo List"));
        assert!(memo_list.contains("Enter open selected memo"));
        assert!(memo_list.contains("y copy selected memo"));
        assert!(memo_list.contains("r refresh memo list"));
        assert!(memo_list.contains("Ctrl+f/PageDown next page"));
        assert!(memo_list.contains(":n next page"));
        assert!(memo_list.contains(":q quit program"));
        assert!(memo_list.contains(":c return to composer"));
        assert!(memo_list.contains("/ enter search"));
    }

    #[test]
    fn memo_list_footer_prefers_status_message() {
        assert_eq!(
            memo_list_footer_text(false, None, Some("loading"), Some("2026-02-26 10:00:00")),
            "[LIST] loading"
        );
    }

    #[test]
    fn memo_list_footer_uses_selected_time_without_status() {
        assert_eq!(
            memo_list_footer_text(false, None, None, Some("2026-02-26 10:00:00")),
            "[LIST] 2026-02-26 10:00:00"
        );
    }

    #[test]
    fn memo_list_footer_is_empty_without_status_or_selection() {
        assert_eq!(memo_list_footer_text(false, None, None, None), "[LIST]");
    }

    #[test]
    fn memo_list_footer_shows_search_query_when_active() {
        assert_eq!(
            memo_list_footer_text(
                false,
                Some("memo"),
                Some("loading"),
                Some("2026-02-26 10:00:00")
            ),
            "[LIST] Search: memo"
        );
    }

    #[test]
    fn memo_list_footer_shows_loading_status_while_fetching() {
        assert_eq!(
            memo_list_footer_text(
                true,
                Some("memo"),
                Some("loading"),
                Some("2026-02-26 10:00:00")
            ),
            "[LIST] Fetching memo list..."
        );
    }
}

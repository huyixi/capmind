use ratatui::style::Color;

const DEFAULT_BG: (u8, u8, u8) = (16, 16, 16);
const COMPOSER_BG: (u8, u8, u8) = (0x41, 0x45, 0x4C);

#[derive(Debug, Clone, Copy)]
pub struct TerminalPalette {
    pub bg: (u8, u8, u8),
}

#[derive(Debug, Clone, Copy)]
pub struct PaneColors {
    pub normal_bg: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct UiTheme {
    pub composer: PaneColors,
}

pub fn detect_terminal_palette() -> TerminalPalette {
    let bg = detect_terminal_bg().unwrap_or(DEFAULT_BG);
    TerminalPalette { bg }
}

pub fn build_ui_theme(palette: TerminalPalette) -> UiTheme {
    let _terminal_bg = palette.bg;

    UiTheme {
        composer: PaneColors {
            normal_bg: to_color(COMPOSER_BG),
        },
    }
}

fn detect_terminal_bg() -> Option<(u8, u8, u8)> {
    parse_rgb_env("CAP_MIND_TUI_BG")
        .or_else(|| parse_colorfgbg())
        .or_else(|| parse_rgb_env("COLORBG"))
}

fn parse_rgb_env(key: &str) -> Option<(u8, u8, u8)> {
    let value = std::env::var(key).ok()?;
    parse_rgb_triplet(&value).or_else(|| parse_hex_rgb(&value))
}

fn parse_colorfgbg() -> Option<(u8, u8, u8)> {
    let value = std::env::var("COLORFGBG").ok()?;
    let bg_index = value
        .split(';')
        .rev()
        .find_map(|piece| piece.trim().parse::<u8>().ok())?;
    Some(ansi256_to_rgb(bg_index))
}

fn parse_rgb_triplet(value: &str) -> Option<(u8, u8, u8)> {
    let normalized = value.replace('/', ",");
    let mut parts = normalized.split(',').map(str::trim);
    let r = parts.next()?.parse::<u8>().ok()?;
    let g = parts.next()?.parse::<u8>().ok()?;
    let b = parts.next()?.parse::<u8>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((r, g, b))
}

fn parse_hex_rgb(value: &str) -> Option<(u8, u8, u8)> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

fn ansi256_to_rgb(value: u8) -> (u8, u8, u8) {
    const ANSI_16: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (128, 0, 0),
        (0, 128, 0),
        (128, 128, 0),
        (0, 0, 128),
        (128, 0, 128),
        (0, 128, 128),
        (192, 192, 192),
        (128, 128, 128),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (0, 0, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];

    match value {
        0..=15 => ANSI_16[value as usize],
        16..=231 => {
            let idx = value - 16;
            let r = idx / 36;
            let g = (idx % 36) / 6;
            let b = idx % 6;
            (cube_to_rgb(r), cube_to_rgb(g), cube_to_rgb(b))
        }
        232..=255 => {
            let gray = 8 + 10 * (value - 232);
            (gray, gray, gray)
        }
    }
}

fn cube_to_rgb(component: u8) -> u8 {
    if component == 0 {
        0
    } else {
        55 + component * 40
    }
}

fn to_color(rgb: (u8, u8, u8)) -> Color {
    Color::Rgb(rgb.0, rgb.1, rgb.2)
}

#[cfg(test)]
mod tests {
    use super::{TerminalPalette, build_ui_theme};

    #[test]
    fn composer_bg_is_fixed_hex_41454c() {
        let light = build_ui_theme(TerminalPalette { bg: (255, 255, 255) });
        let dark = build_ui_theme(TerminalPalette { bg: (0, 0, 0) });

        assert_eq!(light.composer.normal_bg, ratatui::style::Color::Rgb(65, 69, 76));
        assert_eq!(dark.composer.normal_bg, ratatui::style::Color::Rgb(65, 69, 76));
    }

    #[test]
    fn build_ui_theme_returns_rgb_composer_bg() {
        let theme = build_ui_theme(TerminalPalette { bg: (24, 24, 24) });
        assert!(matches!(
            theme.composer.normal_bg,
            ratatui::style::Color::Rgb(_, _, _)
        ));
    }
}

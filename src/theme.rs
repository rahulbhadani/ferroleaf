//! Pink-brown dark theme for Ferroleaf.
//! Iced 0.13 uses closure-based styling — no StyleSheet traits.

use iced::{
    widget::{button, container, scrollable, text_editor, text_input, rule},
    Border, Color, Shadow, Theme, Background,
};

// ── Palette ───────────────────────────────────────────────────────────────────

pub struct Palette;

impl Palette {
    pub const BG_DARKEST:    Color = Color { r: 0.13, g: 0.09, b: 0.09, a: 1.0 };
    pub const BG_DARK:       Color = Color { r: 0.18, g: 0.12, b: 0.12, a: 1.0 };
    pub const BG_MID:        Color = Color { r: 0.22, g: 0.15, b: 0.14, a: 1.0 };
    pub const BG_EDITOR:     Color = Color { r: 0.15, g: 0.10, b: 0.10, a: 1.0 };
    pub const BG_LIGHT:      Color = Color { r: 0.28, g: 0.19, b: 0.18, a: 1.0 };

    pub const PINK_BRIGHT:   Color = Color { r: 0.95, g: 0.55, b: 0.65, a: 1.0 };
    pub const PINK_MID:      Color = Color { r: 0.80, g: 0.42, b: 0.52, a: 1.0 };
    pub const PINK_DIM:      Color = Color { r: 0.55, g: 0.28, b: 0.34, a: 1.0 };

    pub const BROWN_LIGHT:   Color = Color { r: 0.75, g: 0.55, b: 0.48, a: 1.0 };
    pub const BROWN_MID:     Color = Color { r: 0.55, g: 0.38, b: 0.32, a: 1.0 };
    pub const BROWN_DIM:     Color = Color { r: 0.38, g: 0.25, b: 0.21, a: 1.0 };

    pub const TEXT_PRIMARY:  Color = Color { r: 0.95, g: 0.88, b: 0.86, a: 1.0 };
    pub const TEXT_SECONDARY:Color = Color { r: 0.72, g: 0.60, b: 0.57, a: 1.0 };
    pub const TEXT_DIM:      Color = Color { r: 0.50, g: 0.40, b: 0.38, a: 1.0 };
    pub const TEXT_CODE:     Color = Color { r: 0.96, g: 0.90, b: 0.88, a: 1.0 };

    pub const SUCCESS:       Color = Color { r: 0.55, g: 0.85, b: 0.60, a: 1.0 };
    pub const ERROR:         Color = Color { r: 0.95, g: 0.45, b: 0.45, a: 1.0 };
    pub const WARNING:       Color = Color { r: 0.95, g: 0.78, b: 0.35, a: 1.0 };

    // Syntax highlighting
    pub const SYN_COMMAND:   Color = Color { r: 0.95, g: 0.65, b: 0.75, a: 1.0 };
    pub const SYN_BRACE:     Color = Color { r: 0.85, g: 0.70, b: 0.45, a: 1.0 };
    pub const SYN_COMMENT:   Color = Color { r: 0.55, g: 0.45, b: 0.43, a: 1.0 };
    pub const SYN_MATH:      Color = Color { r: 0.65, g: 0.85, b: 0.95, a: 1.0 };
    pub const SYN_STRING:    Color = Color { r: 0.70, g: 0.90, b: 0.65, a: 1.0 };
    pub const SYN_NUMBER:    Color = Color { r: 0.95, g: 0.80, b: 0.55, a: 1.0 };
    pub const SYN_KEYWORD:   Color = Color { r: 0.85, g: 0.55, b: 0.90, a: 1.0 };
}

// ── Container styles (closures) ───────────────────────────────────────────────

fn container_bg(bg: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border::default(),
        shadow: Shadow::default(),
        text_color: None,
    }
}

pub fn sidebar(_t: &Theme) -> container::Style {
    container_bg(Palette::BG_DARK)
}

pub fn toolbar(_t: &Theme) -> container::Style {
    container_bg(Palette::BG_DARKEST)
}

pub fn status_bar(_t: &Theme) -> container::Style {
    container_bg(Palette::BG_DARKEST)
}

pub fn editor_pane(_t: &Theme) -> container::Style {
    container_bg(Palette::BG_EDITOR)
}

pub fn pdf_pane(_t: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Palette::BG_MID)),
        border: Border { color: Palette::BROWN_DIM, width: 1.0, radius: 0.0.into() },
        ..Default::default()
    }
}

pub fn card(_t: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Palette::BG_MID)),
        border: Border { color: Palette::BROWN_MID, width: 1.0, radius: 6.0.into() },
        shadow: Shadow {
            color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.3 },
            offset: iced::Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
        text_color: None,
    }
}

pub fn overlay(_t: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.65 })),
        ..Default::default()
    }
}

pub fn tab_active(_t: &Theme) -> container::Style {
    container_bg(Palette::BG_EDITOR)
}

pub fn tab_inactive(_t: &Theme) -> container::Style {
    container_bg(Palette::BG_DARK)
}

pub fn bg_darkest(_t: &Theme) -> container::Style {
    container_bg(Palette::BG_DARKEST)
}

pub fn bg_mid(_t: &Theme) -> container::Style {
    container_bg(Palette::BG_MID)
}

// ── Button styles ─────────────────────────────────────────────────────────────

pub fn primary_button(_t: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Active | button::Status::Disabled => button::Style {
            background: Some(Background::Color(Palette::PINK_MID)),
            border: Border { radius: 6.0.into(), ..Default::default() },
            text_color: Palette::TEXT_PRIMARY,
            shadow: Shadow {
                color: Color { r: 0.8, g: 0.3, b: 0.4, a: 0.35 },
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 6.0,
            },
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Palette::PINK_BRIGHT)),
            border: Border { radius: 6.0.into(), ..Default::default() },
            text_color: Palette::BG_DARKEST,
            shadow: Shadow {
                color: Color { r: 0.9, g: 0.4, b: 0.5, a: 0.5 },
                offset: iced::Vector::new(0.0, 3.0),
                blur_radius: 10.0,
            },
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Palette::PINK_DIM)),
            border: Border { radius: 6.0.into(), ..Default::default() },
            text_color: Palette::TEXT_PRIMARY,
            shadow: Shadow::default(),
        },
    }
}

pub fn ghost_button(_t: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Active | button::Status::Disabled => button::Style {
            background: None,
            border: Border { radius: 4.0.into(), ..Default::default() },
            text_color: Palette::TEXT_SECONDARY,
            shadow: Shadow::default(),
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Palette::BG_LIGHT)),
            border: Border { radius: 4.0.into(), ..Default::default() },
            text_color: Palette::TEXT_PRIMARY,
            shadow: Shadow::default(),
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Palette::BROWN_DIM)),
            border: Border { radius: 4.0.into(), ..Default::default() },
            text_color: Palette::TEXT_PRIMARY,
            shadow: Shadow::default(),
        },
    }
}

pub fn icon_button(_t: &Theme, status: button::Status) -> button::Style {
    match status {
        button::Status::Active | button::Status::Disabled => button::Style {
            background: None,
            border: Border { radius: 4.0.into(), ..Default::default() },
            text_color: Palette::TEXT_SECONDARY,
            shadow: Shadow::default(),
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Palette::BG_LIGHT)),
            border: Border { radius: 4.0.into(), ..Default::default() },
            text_color: Palette::PINK_BRIGHT,
            shadow: Shadow::default(),
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Palette::BG_DARK)),
            border: Border { radius: 4.0.into(), ..Default::default() },
            text_color: Palette::PINK_MID,
            shadow: Shadow::default(),
        },
    }
}

pub fn file_tree_button(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_t, status| match status {
        button::Status::Active | button::Status::Disabled => button::Style {
            background: if selected { Some(Background::Color(Palette::PINK_DIM)) } else { None },
            border: Border {
                color: if selected { Palette::PINK_MID } else { Color::TRANSPARENT },
                width: if selected { 1.0 } else { 0.0 },
                radius: 3.0.into(),
            },
            text_color: if selected { Palette::PINK_BRIGHT } else { Palette::TEXT_SECONDARY },
            shadow: Shadow::default(),
        },
        button::Status::Hovered | button::Status::Pressed => button::Style {
            background: Some(Background::Color(Palette::BG_LIGHT)),
            border: Border { radius: 3.0.into(), ..Default::default() },
            text_color: Palette::TEXT_PRIMARY,
            shadow: Shadow::default(),
        },
    }
}

pub fn tab_button(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_t, status| match status {
        button::Status::Active | button::Status::Disabled => button::Style {
            background: Some(Background::Color(if active { Palette::BG_EDITOR } else { Palette::BG_DARK })),
            border: Border::default(),
            text_color: if active { Palette::PINK_BRIGHT } else { Palette::TEXT_DIM },
            shadow: Shadow::default(),
        },
        button::Status::Hovered | button::Status::Pressed => button::Style {
            background: Some(Background::Color(Palette::BG_LIGHT)),
            border: Border::default(),
            text_color: Palette::TEXT_PRIMARY,
            shadow: Shadow::default(),
        },
    }
}

// ── Text input ────────────────────────────────────────────────────────────────

pub fn search_input(_t: &Theme, status: text_input::Status) -> text_input::Style {
    let (border_color, border_width) = match status {
        text_input::Status::Focused => (Palette::PINK_MID, 1.5),
        text_input::Status::Hovered => (Palette::BROWN_LIGHT, 1.0),
        _ => (Palette::BROWN_MID, 1.0),
    };
    text_input::Style {
        background: Background::Color(Palette::BG_DARKEST),
        border: Border { color: border_color, width: border_width, radius: 5.0.into() },
        icon: Palette::TEXT_DIM,
        placeholder: Palette::TEXT_DIM,
        value: Palette::TEXT_PRIMARY,
        selection: Palette::PINK_DIM,
    }
}

// ── Text editor ───────────────────────────────────────────────────────────────

pub fn code_editor(_t: &Theme, _status: text_editor::Status) -> text_editor::Style {
    text_editor::Style {
        background: Background::Color(Palette::BG_EDITOR),
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 0.0.into() },
        icon: Palette::TEXT_DIM,
        placeholder: Palette::TEXT_DIM,
        value: Palette::TEXT_CODE,
        selection: Palette::PINK_DIM,
    }
}

// ── Scrollable ────────────────────────────────────────────────────────────────

pub fn dark_scroll(_t: &Theme, status: scrollable::Status) -> scrollable::Style {
    let scroller_color = match status {
        scrollable::Status::Hovered { .. } | scrollable::Status::Dragged { .. } => Palette::PINK_DIM,
        _ => Palette::BROWN_MID,
    };
    scrollable::Style {
        container: container_bg(Palette::BG_DARK),
        vertical_rail: scrollable::Rail {
            background: Some(Background::Color(Palette::BG_DARK)),
            border: Border { radius: 4.0.into(), ..Default::default() },
            scroller: scrollable::Scroller {
                color: scroller_color,
                border: Border { radius: 4.0.into(), ..Default::default() },
            },
        },
        horizontal_rail: scrollable::Rail {
            background: Some(Background::Color(Palette::BG_DARK)),
            border: Border { radius: 4.0.into(), ..Default::default() },
            scroller: scrollable::Scroller {
                color: scroller_color,
                border: Border { radius: 4.0.into(), ..Default::default() },
            },
        },
        gap: None,
    }
}

// ── Rule ─────────────────────────────────────────────────────────────────────

pub fn subtle_rule(_t: &Theme) -> rule::Style {
    rule::Style {
        color: Palette::BROWN_DIM,
        width: 1,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
    }
}

// ── Token colors for syntax highlighting ─────────────────────────────────────

pub fn token_color(kind: &crate::editor::TokenKind) -> Color {
    use crate::editor::TokenKind;
    match kind {
        TokenKind::Command      => Palette::SYN_COMMAND,
        TokenKind::MathInline
        | TokenKind::MathBlock  => Palette::SYN_MATH,
        TokenKind::Comment      => Palette::SYN_COMMENT,
        TokenKind::Brace        => Palette::SYN_BRACE,
        TokenKind::Bracket      => Palette::BROWN_LIGHT,
        TokenKind::Environment  => Palette::SYN_KEYWORD,
        TokenKind::String       => Palette::SYN_STRING,
        TokenKind::Number       => Palette::SYN_NUMBER,
        TokenKind::Normal       => Palette::TEXT_CODE,
    }
}

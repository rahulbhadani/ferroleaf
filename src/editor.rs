use iced::{
    widget::{button, column, container, row, text, text_editor, Space},
    Alignment, Background, Element, Length,
};
use std::path::PathBuf;
use crate::app::Message;
use crate::theme::Palette;

//  Syntax token kinds 

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Command, MathInline, MathBlock, Comment, Brace, Bracket,
    Environment, String, Number, Normal,
}

pub fn tokenize_latex(src: &str) -> Vec<(usize, usize, TokenKind)> {
    let bytes = src.as_bytes();
    let len = bytes.len();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < len {
        let c = bytes[i] as char;
        if c == '%' {
            let s = i;
            while i < len && bytes[i] != b'\n' { i += 1; }
            tokens.push((s, i, TokenKind::Comment)); continue;
        }
        if c == '\\' {
            let s = i; i += 1;
            if i < len && (bytes[i] as char).is_alphabetic() {
                while i < len && (bytes[i] as char).is_alphanumeric() { i += 1; }
            } else if i < len { i += 1; }
            tokens.push((s, i, TokenKind::Command)); continue;
        }
        if c == '$' {
            let s = i; i += 1;
            let block = i < len && bytes[i] == b'$';
            if block { i += 1; }
            let end_seq = if block { "$$" } else { "$" };
            while i < len {
                if src[i..].starts_with(end_seq) { i += end_seq.len(); break; }
                i += 1;
            }
            tokens.push((s, i, if block { TokenKind::MathBlock } else { TokenKind::MathInline }));
            continue;
        }
        if c == '{' || c == '}' { tokens.push((i, i+1, TokenKind::Brace)); i += 1; continue; }
        if c == '[' || c == ']' { tokens.push((i, i+1, TokenKind::Bracket)); i += 1; continue; }
        if c.is_ascii_digit() {
            let s = i;
            while i < len && ((bytes[i] as char).is_ascii_digit() || bytes[i] == b'.') { i += 1; }
            tokens.push((s, i, TokenKind::Number)); continue;
        }
        i += 1;
    }
    tokens
}

//  Per-line tokenizer used by the Highlighter trait 

fn tokenize_line_hl(
    line: &str,
    in_math_block: &mut bool,
) -> Vec<(std::ops::Range<usize>, TokenKind)> {
    let bytes = line.as_bytes();
    let len   = bytes.len();
    let mut tokens: Vec<(std::ops::Range<usize>, TokenKind)> = Vec::new();
    let mut i = 0;

    // Continue consuming an open $$ block from the previous line
    if *in_math_block {
        let s = 0;
        while i < len {
            if i + 1 < len && bytes[i] == b'$' && bytes[i + 1] == b'$' {
                i += 2;
                *in_math_block = false;
                break;
            }
            i += 1;
        }
        tokens.push((s..i, TokenKind::MathBlock));
    }

    while i < len {
        let c = bytes[i] as char;

        // Comment: % to end of line
        if c == '%' {
            tokens.push((i..len, TokenKind::Comment));
            break;
        }

        // LaTeX command: \cmd or \@
        if c == '\\' {
            let s = i;
            i += 1;
            if i < len && (bytes[i] as char).is_alphabetic() {
                while i < len && (bytes[i] as char).is_alphanumeric() { i += 1; }
                let cmd = &line[s + 1..i];
                let kind = if cmd == "begin" || cmd == "end" {
                    TokenKind::Environment
                } else {
                    TokenKind::Command
                };
                tokens.push((s..i, kind));
            } else if i < len {
                i += 1;
                tokens.push((s..i, TokenKind::Command));
            }
            continue;
        }

        // Math: $$ (block) or $ (inline)
        if c == '$' {
            let s = i;
            i += 1;
            let block = i < len && bytes[i] == b'$';
            if block {
                i += 1;
                let mut closed = false;
                while i + 1 < len {
                    if bytes[i] == b'$' && bytes[i + 1] == b'$' {
                        i += 2;
                        closed = true;
                        break;
                    }
                    i += 1;
                }
                if !closed { i = len; *in_math_block = true; }
                tokens.push((s..i, TokenKind::MathBlock));
            } else {
                while i < len && bytes[i] != b'$' && bytes[i] != b'\n' { i += 1; }
                if i < len && bytes[i] == b'$' { i += 1; }
                tokens.push((s..i, TokenKind::MathInline));
            }
            continue;
        }

        // Braces & brackets
        if matches!(c, '{' | '}') { tokens.push((i..i + 1, TokenKind::Brace));   i += 1; continue; }
        if matches!(c, '[' | ']') { tokens.push((i..i + 1, TokenKind::Bracket)); i += 1; continue; }

        // Numbers
        if c.is_ascii_digit() {
            let s = i;
            while i < len && ((bytes[i] as char).is_ascii_digit() || bytes[i] == b'.') { i += 1; }
            tokens.push((s..i, TokenKind::Number));
            continue;
        }

        i += 1;
    }

    tokens
}

//  Highlighter settings & implementation 

/// Unit settings — no configuration needed for LaTeX highlighting.
#[derive(Debug, Clone, PartialEq)]
pub struct LatexHighlightSettings;

/// Stateful per-document highlighter that tracks multi-line math blocks.
pub struct LatexHighlighter {
    current_line:  usize,
    in_math_block: bool,
}

impl iced::advanced::text::highlighter::Highlighter for LatexHighlighter {
    type Settings  = LatexHighlightSettings;
    type Highlight = TokenKind;
    type Iterator<'a> = std::vec::IntoIter<(std::ops::Range<usize>, TokenKind)>;

    fn new(_settings: &Self::Settings) -> Self {
        LatexHighlighter { current_line: 0, in_math_block: false }
    }

    fn update(&mut self, _new: &Self::Settings) {}

    fn change_line(&mut self, line: usize) {
        self.current_line = line;
        // Reset per-call in case the viewer starts at a non-zero line
        if line == 0 { self.in_math_block = false; }
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        tokenize_line_hl(line, &mut self.in_math_block).into_iter()
    }

    fn current_line(&self) -> usize { self.current_line }
}

/// Map a `TokenKind` to an iced highlight format (color + optional font override).
pub fn latex_highlight_format(
    highlight: &TokenKind,
    _theme:    &iced::Theme,
) -> iced::advanced::text::highlighter::Format<iced::Font> {
    use iced::Color;
    use iced::advanced::text::highlighter::Format;
    let color = match highlight {
        TokenKind::Command     => Some(Color { r: 0.56, g: 0.74, b: 0.95, a: 1.0 }), // sky-blue
        TokenKind::Environment => Some(Color { r: 0.72, g: 0.52, b: 0.96, a: 1.0 }), // violet
        TokenKind::Comment     => Some(Color { r: 0.48, g: 0.70, b: 0.48, a: 1.0 }), // muted green
        TokenKind::MathInline |
        TokenKind::MathBlock   => Some(Color { r: 0.95, g: 0.82, b: 0.42, a: 1.0 }), // amber
        TokenKind::Brace       => Some(Color { r: 0.95, g: 0.55, b: 0.65, a: 1.0 }), // pink
        TokenKind::Bracket     => Some(Color { r: 0.90, g: 0.65, b: 0.38, a: 1.0 }), // orange
        TokenKind::Number      => Some(Color { r: 0.78, g: 0.92, b: 0.68, a: 1.0 }), // lime
        TokenKind::String |
        TokenKind::Normal      => None, // inherit default editor color
    };
    Format { color, font: None }
}

//  Editor state 

//  Editor state 

pub struct EditorState {
    pub content: text_editor::Content,
    pub path: PathBuf,
    /// Undo history: each entry is the full document text before an edit.
    undo_stack: Vec<String>,
    /// Redo history: rebuilt when undo is called.
    redo_stack: Vec<String>,
}

impl EditorState {
    pub fn new(path: PathBuf, text: &str) -> Self {
        EditorState {
            content: text_editor::Content::with_text(text),
            path,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }
    pub fn text(&self) -> String { self.content.text() }
    pub fn cursor_position(&self) -> (usize, usize) { self.content.cursor_position() }

    /// Call before any edit that should be undoable.
    pub fn snapshot(&mut self) {
        let t = self.text();
        // Don't push duplicate snapshots.
        if self.undo_stack.last().map(|s| s == &t).unwrap_or(false) { return; }
        self.undo_stack.push(t);
        // Committing a new edit clears the redo stack.
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.text());
            self.content = text_editor::Content::with_text(&prev);
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.text());
            self.content = text_editor::Content::with_text(&next);
        }
    }
    /// Move the editor cursor to the given 1-based line number.
    ///
    /// iced 0.13 has no direct "scroll to line" API, but Content::perform
    /// accepts Action::Move(Motion) calls that reposition the cursor and
    /// cause the widget to scroll the cursor into view on the next frame.
    ///
    /// Strategy: jump to document start, walk down (line-1) rows,
    /// then End+Home so the whole line is visible and cursor is at column 0.
    pub fn jump_to_line(&mut self, line: u32) {
        use text_editor::{Action, Motion};
        let target = line.max(1) as usize;
        // 1. Move to document start (clears selection).
        self.content.perform(Action::Move(Motion::DocumentStart));
        // 2. Walk down (target - 1) lines.
        for _ in 1..target {
            self.content.perform(Action::Move(Motion::Down));
        }
        // 3. End then Home: scrolls the line into view, cursor rests at col 0.
        self.content.perform(Action::Move(Motion::End));
        self.content.perform(Action::Move(Motion::Home));
    }
}

//  Tab bar 

pub fn tab_bar<'a>(
    open_files: &'a [PathBuf],
    active: Option<&'a PathBuf>,
    dirty: impl Fn(&PathBuf) -> bool,
) -> Element<'a, Message> {
    let tabs: Vec<Element<Message>> = open_files.iter().map(|path| {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        let is_active = active == Some(path);
        let dot = if dirty(path) { "* " } else { "" };

        let close_btn = button(text("x").size(13).color(Palette::TEXT_DIM))
            .on_press(Message::CloseTab(path.clone()))
            .style(crate::theme::ghost_button)
            .padding([0u16, 4u16]);

        let tab_btn = button(
            row![
                text(format!("{}{}", dot, name)).size(13)
                    .color(if is_active { Palette::TEXT_PRIMARY } else { Palette::TEXT_DIM }),
                close_btn,
            ].spacing(4).align_y(Alignment::Center)
        )
        .on_press(Message::SwitchTab(path.clone()))
        .style(crate::theme::tab_button(is_active))
        .padding([6u16, 12u16]);

        // Pink underline for active tab
        let underline_color = if is_active { Palette::PINK_MID } else { iced::Color::TRANSPARENT };
        container(
            column![
                tab_btn,
                container(Space::with_height(2))
                    .style(move |_t: &iced::Theme| iced::widget::container::Style {
                        background: Some(iced::Background::Color(underline_color)),
                        ..Default::default()
                    })
                    .width(Length::Fill),
            ].spacing(0)
        )
        .style(if is_active { crate::theme::tab_active } else { crate::theme::tab_inactive })
        .into()
    }).collect();

    container(row(tabs).spacing(1).align_y(Alignment::End))
        .width(Length::Fill)
        .height(38)
        .style(crate::theme::toolbar)
        .into()
}

// ─── Current-line highlight column ───────────────────────────────────────────

/// Builds a full-width column that sits as the *bottom* layer of a `stack!`.
///
/// Each row is exactly `font_size * 1.3` px tall — matching iced's default
/// `LineHeight::Relative(1.3)` — so they align 1-to-1 with editor lines.
/// The active row is given a warm highlight background.
/// Hovering over the highlighted row shows a tooltip with the line number.
pub fn line_highlight_column(
    line_count: usize,
    font_size:  u16,
    cursor_line: usize,
) -> Element<'static, Message> {
    let line_h    = (font_size as f32 * 1.3).round();
    let highlight = iced::Color { r: 0.28, g: 0.19, b: 0.18, a: 1.0 };

    let rows: Vec<Element<Message>> = (0..line_count.max(1))
        .map(|i| {
            let is_active = i == cursor_line;
            container(Space::new(Length::Fill, Length::Fixed(line_h)))
                .width(Length::Fill)
                .style(move |_: &iced::Theme| iced::widget::container::Style {
                    background: if is_active {
                        Some(Background::Color(highlight))
                    } else {
                        None
                    },
                    ..Default::default()
                })
                .into()
        })
        .collect();

    container(column(rows).spacing(0))
        .padding(iced::Padding { top: 4.0, right: 0.0, bottom: 0.0, left: 0.0 })
        .width(Length::Fill)
        .into()
}

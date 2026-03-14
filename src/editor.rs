use iced::{
    widget::{button, column, container, row, text, text_editor, Space},
    Alignment, Element, Font, Length,
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

//  Editor state 

pub struct EditorState {
    pub content: text_editor::Content,
    pub path: PathBuf,
}

impl EditorState {
    pub fn new(path: PathBuf, text: &str) -> Self {
        EditorState { content: text_editor::Content::with_text(text), path }
    }
    pub fn text(&self) -> String { self.content.text() }
    pub fn cursor_position(&self) -> (usize, usize) { self.content.cursor_position() }
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
        let dot = if dirty(path) { "● " } else { "" };

        let close_btn = button(text("×").size(13).color(Palette::TEXT_DIM))
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

//  Line number gutter 

pub fn line_gutter(line_count: usize, _scroll_offset: f32, _viewport_height: f32) -> Element<'static, Message> {
    let numbers: Vec<Element<Message>> = (1..=(line_count.max(1)))
        .map(|n| {
            text(format!("{:>4}", n))
                .size(12)
                .font(Font::MONOSPACE)
                .color(Palette::TEXT_DIM)
                .into()
        })
        .collect();

    container(column(numbers).spacing(2))
        .width(48)
        // Use [u16; 2] padding — top/bottom, left/right
        .padding([4u16, 4u16])
        .style(crate::theme::editor_pane)
        .into()
}

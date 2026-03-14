use iced::{
    widget::{button, column, container, row, scrollable, text, Space},
    Alignment, Element, Length,
};
use std::collections::HashSet;
use std::path::PathBuf;
use crate::project::FileEntry;
use crate::theme::Palette;
use crate::app::Message;

#[derive(Debug, Clone, Default)]
pub struct FileTree {
    pub expanded_dirs: HashSet<PathBuf>,
}

impl FileTree {
    pub fn toggle_dir(&mut self, path: &PathBuf) {
        if self.expanded_dirs.contains(path) {
            self.expanded_dirs.remove(path);
        } else {
            self.expanded_dirs.insert(path.clone());
        }
    }

    /// Takes owned entries — no borrow escapes into the returned Element.
    pub fn view<'a>(
        &'a self,
        entries: Vec<FileEntry>,
        active_file: Option<&'a PathBuf>,
        main_file: Option<&'a PathBuf>,
        search_query: &'a str,
    ) -> Element<'a, Message> {
        let mut col = column![].spacing(1).padding([2u16, 0u16]);
        let searching = !search_query.is_empty();

        for entry in &entries {
            // Visibility: depth-0 always shown; deeper entries need all ancestors expanded
            if entry.depth > 0 && !searching {
                let mut all_expanded = true;
                let mut ancestor = entry.path.parent();
                loop {
                    match ancestor {
                        Some(p) if p != entry.path => {
                            if !self.expanded_dirs.contains(p) {
                                all_expanded = false;
                                break;
                            }
                            ancestor = p.parent();
                        }
                        _ => break,
                    }
                }
                if !all_expanded { continue; }
            }

            // Search filter on files only
            if searching && !entry.is_dir {
                if !entry.name.to_lowercase().contains(&search_query.to_lowercase()) {
                    continue;
                }
            }

            let indent = (entry.depth * 14) as u16;
            let is_tex = !entry.is_dir
                && entry.path.extension().and_then(|e| e.to_str()) == Some("tex");
            let is_main = main_file == Some(&entry.path);
            let is_active = !entry.is_dir && active_file == Some(&entry.path);

            let icon: &str = if entry.is_dir {
                if self.expanded_dirs.contains(&entry.path) { "▾" } else { "▸" }
            } else {
                file_icon(&entry.name)
            };

            let name_color = if is_main {
                Palette::PINK_BRIGHT
            } else if is_active {
                Palette::TEXT_PRIMARY
            } else if entry.is_dir {
                Palette::BROWN_LIGHT
            } else {
                Palette::TEXT_SECONDARY
            };

            let open_msg = if entry.is_dir {
                Message::ToggleFileTreeDir(entry.path.clone())
            } else {
                Message::OpenFile(entry.path.clone())
            };

            // Main label button (takes up all space left of the star button)
            let label_btn = button(
                row![
                    Space::with_width(indent),
                    text(icon).size(11u16).color(Palette::TEXT_DIM),
                    Space::with_width(4u16),
                    text(entry.name.clone()).size(13u16).color(name_color),
                ].align_y(Alignment::Center)
            )
            .on_press(open_msg)
            .style(crate::theme::file_tree_button(is_active && !entry.is_dir))
            .width(Length::Fill)
            .padding([3u16, 4u16]);

            // "Set as main" star button — only on .tex files that aren't already main
            let row_widget: Element<Message> = if is_tex {
                let star_color = if is_main { Palette::PINK_BRIGHT } else { Palette::TEXT_DIM };
                let star_msg = if is_main {
                    Message::Noop // already main, clicking does nothing
                } else {
                    Message::SetMainFile(entry.path.clone())
                };
                row![
                    label_btn,
                    button(text("★").size(11u16).color(star_color))
                        .on_press(star_msg)
                        .style(crate::theme::ghost_button)
                        .padding([2u16, 6u16]),
                ].align_y(Alignment::Center).into()
            } else {
                label_btn.into()
            };

            col = col.push(row_widget);
        }

        scrollable(container(col).width(Length::Fill))
            .height(Length::Fill)
            .style(crate::theme::dark_scroll)
            .into()
    }
}

fn file_icon(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "tex"                                  => "⊤",
        "bib"                                  => "⊛",
        "cls" | "sty"                          => "⊞",
        "pdf"                                  => "⊡",
        "png" | "jpg" | "jpeg" | "svg" | "eps" => "⊠",
        "txt" | "md"                           => "≡",
        _                                      => "·",
    }
}

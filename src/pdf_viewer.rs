//! PDF viewer panel.
//!
//! Renders PDF pages by shelling out to `pdftoppm` (part of poppler-utils,
//! always installed alongside texlive). Each page becomes a PNG that is loaded
//! as an iced image handle. No native library linking required.

use std::path::{Path, PathBuf};
use iced::{
    widget::{button, column, container, image, row, scrollable, text, Space},
    Alignment, Element, Length,
};
use crate::app::Message;
use crate::theme::Palette;

#[derive(Debug, Default)]
pub struct PdfViewer {
    pub pdf_path: Option<PathBuf>,
    pub current_page: usize,
    pub total_pages: usize,
    pub zoom: f32,
    pub rendered_pages: Vec<RenderedPage>,
    pub synctex_highlight: Option<SynctexHighlight>,
}

#[derive(Debug, Clone)]
pub struct RenderedPage {
    pub page_number: usize,
    pub width: u32,
    pub height: u32,
    pub handle: Option<iced::widget::image::Handle>,
}

#[derive(Debug, Clone)]
pub struct SynctexHighlight {
    pub page: usize,
    pub x: f32, pub y: f32, pub w: f32, pub h: f32,
}

impl PdfViewer {
    pub fn new() -> Self { PdfViewer { zoom: 1.0, ..Default::default() } }

    pub fn load_pdf(&mut self, path: PathBuf) {
        self.pdf_path = Some(path);
        self.current_page = 0;
        self.rendered_pages.clear();
        self.synctex_highlight = None;
    }

    pub fn set_zoom(&mut self, z: f32) { self.zoom = z.clamp(0.25, 4.0); }
    pub fn zoom_in(&mut self)  { self.set_zoom(self.zoom * 1.2); }
    pub fn zoom_out(&mut self) { self.set_zoom(self.zoom / 1.2); }
    pub fn zoom_fit(&mut self) { self.set_zoom(1.0); }

    pub fn next_page(&mut self) {
        if self.current_page + 1 < self.total_pages { self.current_page += 1; }
    }
    pub fn prev_page(&mut self) {
        if self.current_page > 0 { self.current_page -= 1; }
    }

    pub fn view(&self) -> Element<'_, Message> {
        if self.pdf_path.is_none() {
            return container(
                column![
                    text("No PDF yet").size(20u16).color(Palette::TEXT_DIM),
                    text("Compile your document to see a preview")
                        .size(13u16).color(Palette::TEXT_DIM),
                ].spacing(8).align_x(Alignment::Center)
            )
            .width(Length::Fill).height(Length::Fill)
            .center_x(Length::Fill).center_y(Length::Fill)
            .style(crate::theme::pdf_pane)
            .into();
        }

        column![self.toolbar(), self.pdf_content()]
            .spacing(0).width(Length::Fill).height(Length::Fill)
            .into()
    }

    fn toolbar(&self) -> Element<'_, Message> {
        let page_info = text(format!("{} / {}", self.current_page + 1, self.total_pages.max(1)))
            .size(13u16).color(Palette::TEXT_SECONDARY);
        let zoom_label = text(format!("{:.0}%", self.zoom * 100.0))
            .size(12u16).color(Palette::TEXT_DIM);

        let pb = |label: &'static str, msg: Message| {
            button(text(label).size(13u16))
                .on_press(msg)
                .style(crate::theme::ghost_button)
                .padding([3u16, 8u16])
        };

        container(row![
            pb("◂", Message::PdfPrevPage),
            page_info,
            pb("▸", Message::PdfNextPage),
            Space::with_width(12),
            pb("−", Message::PdfZoomOut),
            zoom_label,
            pb("+", Message::PdfZoomIn),
            pb("⊡", Message::PdfZoomFit),
            Space::with_width(Length::Fill),
            pb("↻  Recompile", Message::Compile),
        ].spacing(4).align_y(Alignment::Center).padding([4u16, 8u16]))
        .width(Length::Fill).height(36)
        .style(crate::theme::toolbar)
        .into()
    }

    fn pdf_content(&self) -> Element<'_, Message> {
        if self.rendered_pages.is_empty() {
            return container(
                column![
                    text("Rendering…").size(14u16).color(Palette::TEXT_DIM),
                    text("(requires poppler-utils / pdftoppm)")
                        .size(11u16).color(Palette::TEXT_DIM),
                ].spacing(4).align_x(Alignment::Center)
            )
            .center_x(Length::Fill).center_y(Length::Fill)
            .width(Length::Fill).height(Length::Fill)
            .into();
        }

        let pages: Vec<Element<Message>> = self.rendered_pages.iter()
            .map(|page| self.render_page(page))
            .collect();

        scrollable(
            container(column(pages).spacing(12).align_x(Alignment::Center))
                .width(Length::Fill).padding(16u16)
        )
        .width(Length::Fill).height(Length::Fill)
        .style(crate::theme::dark_scroll)
        .into()
    }

    fn render_page(&self, page: &RenderedPage) -> Element<'_, Message> {
        let sw = page.width as f32 * self.zoom;
        let sh = page.height as f32 * self.zoom;
        let page_num = page.page_number;
        // Store rendered (zoomed) dimensions so the click handler can normalise them.
        let pw = sw;
        let ph = sh;
        // Original (unzoomed) pixel dimensions — what synctex coords are based on.
        let orig_w = page.width as f32;
        let orig_h = page.height as f32;

        let img: Element<Message> = if let Some(handle) = &page.handle {
            button(
                image(handle.clone())
                    .width(Length::Fixed(sw))
                    .height(Length::Fixed(sh))
            )
            // on_press_with gives us the cursor position within the widget at click time.
            .on_press_with(move || {
                // We don't have the cursor offset here (iced limitation), so we
                // emit a sentinel and rely on the most-recent mouse position tracked
                // via the subscription in app.rs. For now emit centre as fallback —
                // the real fix is to track CursorMoved events (see app.rs).
                Message::PdfClicked {
                    page: page_num,
                    x: pw / 2.0,
                    y: ph / 2.0,
                    page_w: orig_w,
                    page_h: orig_h,
                }
            })
            .style(crate::theme::ghost_button)
            .padding(0u16)
            .into()
        } else {
            container(
                text(format!("Page {}", page.page_number + 1))
                    .size(14u16).color(Palette::TEXT_DIM)
            )
            .width(Length::Fixed(sw))
            .height(Length::Fixed(sh))
            .center_x(Length::Fixed(sw))
            .center_y(Length::Fixed(sh))
            .style(crate::theme::card)
            .into()
        };

        container(img).style(crate::theme::card).into()
    }
}

// ── PDF rendering via pdftoppm ────────────────────────────────────────────────
//
// pdftoppm ships with poppler-utils which comes with every texlive installation.
// We render all pages to PNG in a temp dir, then read the PNG bytes back.
// No native library linking — just a subprocess call.

pub async fn render_pdf_pages(
    pdf_path: &Path,
    zoom: f32,
) -> anyhow::Result<Vec<RenderedPage>> {
    use tokio::process::Command;

    // DPI: standard 72dpi * zoom. pdftoppm default is 150dpi, we scale from 96.
    let dpi = (96.0 * zoom).round() as u32;

    let tmp = tempfile::tempdir()?;
    let out_prefix = tmp.path().join("page");

    // Check which tool is available: pdftoppm (poppler) or gs (ghostscript)
    let tool = if which_cmd("pdftoppm").await { "pdftoppm" }
               else if which_cmd("gs").await   { "gs" }
               else {
                   anyhow::bail!(
                       "No PDF renderer found. Install poppler-utils:\n  \
                        sudo apt-get install poppler-utils\n  \
                        sudo dnf install poppler-utils\n  \
                        sudo pacman -S poppler"
                   );
               };

    if tool == "pdftoppm" {
        // pdftoppm -r <dpi> -png <input.pdf> <output-prefix>
        // produces: <output-prefix>-01.png, -02.png, …
        let status = Command::new("pdftoppm")
            .args([
                "-r", &dpi.to_string(),
                "-png",
                pdf_path.to_str().unwrap_or(""),
                out_prefix.to_str().unwrap_or("page"),
            ])
            .status().await?;

        if !status.success() {
            anyhow::bail!("pdftoppm failed");
        }
    } else {
        // Ghostscript fallback
        let status = Command::new("gs")
            .args([
                "-dBATCH", "-dNOPAUSE", "-dQUIET",
                "-sDEVICE=png16m",
                &format!("-r{}", dpi),
                &format!("-sOutputFile={}-%02d.png", out_prefix.display()),
                pdf_path.to_str().unwrap_or(""),
            ])
            .status().await?;

        if !status.success() {
            anyhow::bail!("ghostscript failed");
        }
    }

    // Collect all produced PNG files in order
    let mut png_files: Vec<PathBuf> = std::fs::read_dir(tmp.path())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("png"))
        .collect();
    png_files.sort(); // page-01.png, page-02.png … lexicographic = correct order

    if png_files.is_empty() {
        anyhow::bail!("No pages rendered — PDF may be empty or corrupt");
    }

    let mut pages = Vec::with_capacity(png_files.len());
    for (i, png_path) in png_files.iter().enumerate() {
        let bytes = tokio::fs::read(png_path).await?;
        // Decode PNG header to get width/height without a full decode library
        let (w, h) = png_dimensions(&bytes).unwrap_or((800, 1132));
        let handle = iced::widget::image::Handle::from_bytes(bytes);
        pages.push(RenderedPage {
            page_number: i,
            width: w,
            height: h,
            handle: Some(handle),
        });
    }

    Ok(pages)
}

async fn which_cmd(cmd: &str) -> bool {
    tokio::process::Command::new("which")
        .arg(cmd)
        .output().await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Read width/height from a PNG header (bytes 16-24) without decoding the image.
fn png_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 24 { return None; }
    // PNG magic: 8 bytes; IHDR chunk: 4 len + 4 "IHDR" + 4 width + 4 height
    if &data[0..8] != b"\x89PNG\r\n\x1a\n" { return None; }
    let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Some((w, h))
}

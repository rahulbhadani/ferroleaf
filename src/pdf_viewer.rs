//! PDF viewer panel.
//!
//! Renders PDF pages by shelling out to `pdftoppm` (part of poppler-utils,
//! always installed alongside texlive). Each page becomes a PNG that is loaded
//! as an iced image handle. No native library linking required.

use std::path::{Path, PathBuf};
use iced::{
    widget::{button, column, container, image, row, scrollable, text, tooltip, Space},
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
    /// Unscaled width (pixels at zoom=1.0) of the first page.
    /// Set once on initial load, used by Fit to avoid dividing by a stale zoom.
    pub base_page_width: Option<u32>,
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
    pub fn new() -> Self { PdfViewer { zoom: 1.0, base_page_width: None, ..Default::default() } }

    pub fn load_pdf(&mut self, path: PathBuf) {
        self.pdf_path = Some(path);
        self.current_page = 0;
        self.rendered_pages.clear();
        self.synctex_highlight = None;
        self.base_page_width = None; // will be set from first zoom=1.0 render
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

        let tb = |label: &'static str, tip_str: &'static str, msg: Message|
            -> Element<'_, Message>
        {
            tooltip(
                button(text(label).size(13u16))
                    .on_press(msg)
                    .style(crate::theme::ghost_button)
                    .padding([3u16, 8u16]),
                container(text(tip_str).size(11u16).color(Palette::TEXT_PRIMARY))
                    .padding([4u16, 8u16])
                    .style(crate::theme::tooltip_box),
                tooltip::Position::Bottom,
            )
            .into()
        };

        container(row![
            tb("<",         "Previous Page",           Message::PdfPrevPage),
            page_info,
            tb(">",         "Next Page",               Message::PdfNextPage),
            Space::with_width(12),
            tb("-",         "Zoom Out",                Message::PdfZoomOut),
            zoom_label,
            tb("+",         "Zoom In",                 Message::PdfZoomIn),
            tb("Fit",       "Fit Page to Panel Width", Message::PdfZoomFit),
            Space::with_width(Length::Fill),
            tb("Recompile", "Recompile Document",      Message::Compile),
        ].spacing(4).align_y(Alignment::Center).padding([4u16, 8u16]))
        .width(Length::Fill).height(36)
        .style(crate::theme::toolbar)
        .into()
    }

    fn pdf_content(&self) -> Element<'_, Message> {
        if self.rendered_pages.is_empty() {
            return container(
                column![
                    text("Rendering...").size(14u16).color(Palette::TEXT_DIM),
                    text("(requires poppler-utils / pdftoppm)")
                        .size(11u16).color(Palette::TEXT_DIM),
                ].spacing(4).align_x(Alignment::Center)
            )
            .center_x(Length::Fill).center_y(Length::Fill)
            .width(Length::Fill).height(Length::Fill)
            .into();
        }

        // Show only the current page (lazy rendering — other pages are re-rendered
        // on demand when navigating, so there is no need to keep all in the DOM).
        let page = &self.rendered_pages[self.current_page.min(self.rendered_pages.len() - 1)];

        scrollable(
            container(self.render_page(page))
                .width(Length::Fill)
                .center_x(Length::Fill)
                .padding(16u16)
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

//  PDF rendering via pdftoppm 
//
// pdftoppm ships with poppler-utils which comes with every texlive installation.
// We render all pages to PNG in a temp dir, then read the PNG bytes back.
// No native library linking — just a subprocess call.

pub async fn render_pdf_pages(
    pdf_path: &Path,
    zoom: f32,
) -> anyhow::Result<Vec<RenderedPage>> {
    render_pdf_page_range(pdf_path, zoom, None).await
}

/// Render only a specific 1-based page range (inclusive).
/// Pass None to render all pages.
pub async fn render_pdf_page_range(
    pdf_path: &Path,
    zoom: f32,
    page_range: Option<(u32, u32)>,
) -> anyhow::Result<Vec<RenderedPage>> {
    use tokio::process::Command;

    let dpi = (96.0 * zoom).round() as u32;

    let tmp = tempfile::tempdir()?;
    let out_prefix = tmp.path().join("page");

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
        let mut args = vec![
            "-r".to_string(), dpi.to_string(),
            "-png".to_string(),
        ];
        if let Some((first, last)) = page_range {
            args.push("-f".to_string()); args.push(first.to_string());
            args.push("-l".to_string()); args.push(last.to_string());
        }
        args.push(pdf_path.to_str().unwrap_or("").to_string());
        args.push(out_prefix.to_str().unwrap_or("page").to_string());

        let status = Command::new("pdftoppm")
            .args(&args)
            .status().await?;
        if !status.success() {
            anyhow::bail!("pdftoppm failed");
        }
    } else {
        // Ghostscript fallback — page range via -dFirstPage / -dLastPage
        let mut args = vec![
            "-dBATCH".to_string(), "-dNOPAUSE".to_string(), "-dQUIET".to_string(),
            "-sDEVICE=png16m".to_string(),
            format!("-r{}", dpi),
            format!("-sOutputFile={}-%02d.png", out_prefix.display()),
        ];
        if let Some((first, last)) = page_range {
            args.push(format!("-dFirstPage={}", first));
            args.push(format!("-dLastPage={}", last));
        }
        args.push(pdf_path.to_str().unwrap_or("").to_string());

        let status = Command::new("gs")
            .args(&args)
            .status().await?;
        if !status.success() {
            anyhow::bail!("ghostscript failed");
        }
    }

    let mut png_files: Vec<PathBuf> = std::fs::read_dir(tmp.path())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("png"))
        .collect();
    png_files.sort();

    if png_files.is_empty() {
        anyhow::bail!("No pages rendered -- PDF may be empty or corrupt");
    }

    let page_offset = page_range.map(|(f, _)| (f - 1) as usize).unwrap_or(0);
    let mut pages = Vec::with_capacity(png_files.len());
    for (i, png_path) in png_files.iter().enumerate() {
        let bytes = tokio::fs::read(png_path).await?;
        let (w, h) = png_dimensions(&bytes).unwrap_or((800, 1132));
        let handle = iced::widget::image::Handle::from_bytes(bytes);
        pages.push(RenderedPage {
            page_number: page_offset + i,
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

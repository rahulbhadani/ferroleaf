//! Main application state, update, and view for Ferroleaf.
//! Iced 0.13 functional API — Task replaces Command, no Application trait.

use iced::{
    keyboard, mouse,
    widget::{
        button, column, container, horizontal_rule, row, scrollable,
        stack, svg, text, text_editor, text_input, tooltip, Space,
    },
    Alignment, Element, Event, Font, Length, Subscription, Task, Theme,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::compiler::{CompileOptions, CompileStatus, Compiler, CompilerKind, DiagnosticLevel};
use crate::editor::{tab_bar, EditorState, LatexHighlighter, LatexHighlightSettings, latex_highlight_format};
use crate::file_tree::FileTree;
use crate::pdf_viewer::PdfViewer;
use crate::project::{Project, ProjectSettings};
use crate::synctex;
use crate::theme::Palette;

//  Menu & context-menu types 

#[derive(Debug, Clone, PartialEq)]
pub enum MenuKind { File, Edit, Build, View, Help }

#[derive(Debug, Clone, PartialEq)]
pub enum ContextMenuKind { Editor, PdfViewer }

#[derive(Debug, Clone)]
pub struct ContextMenuState {
    pub kind:   ContextMenuKind,
    pub x:      f32,
    pub y:      f32,
    pub page_w: f32,  // captured rendered-page width (PDF context only)
    pub page_h: f32,
}

//  Messages 

#[derive(Debug, Clone)]
pub enum Message {
    OpenProject, NewProject,
    ProjectOpened(Option<PathBuf>),
    OpenFile(PathBuf), CloseTab(PathBuf), SwitchTab(PathBuf),
    ToggleFileTreeDir(PathBuf),
    NewFile, NewFileNameChanged(String), NewFileConfirmed, NewFileCancelled,
    SetMainFile(PathBuf), DeleteFile(PathBuf),
    EditorAction(text_editor::Action),
    /// Mouse wheel scrolled — used to keep the line-number gutter in sync
    /// with the text_editor's internal scroll position.
    EditorWheelScrolled(mouse::ScrollDelta),
    SaveFile, SaveAll, SearchChanged(String),
    Compile,
    CompileStatusUpdate(Arc<CompileStatus>),
    CompileOptionChanged(CompileOptionMsg),
    ClearLog,
    PdfPagesRendered(Vec<crate::pdf_viewer::RenderedPage>),
    MouseMoved(f32, f32),
    PdfClicked { page: usize, x: f32, y: f32, page_w: f32, page_h: f32 },
    PdfNextPage, PdfPrevPage, PdfZoomIn, PdfZoomOut, PdfZoomFit,
    SynctexResult(Option<synctex::SourceLocation>),
    GoToLine(u32),
    ToggleSidebar, ToggleLogPanel, ShowSettings, CloseSettings,
    KeyPressed(keyboard::Key, keyboard::Modifiers),
    WindowResized(f32, f32),
    //  Menu bar & context menu 
    ToggleMenu(MenuKind),
    Dismiss,
    RightClicked,
    SynctexAt { page: usize, x: f32, y: f32, page_w: f32, page_h: f32 },
    //  Editor actions 
    CommentLine,
    SelectAll,
    CloseActiveTab,
    //  App control 
    Quit,
    About,
    ShowKeyboardShortcuts,
    Noop, Error(String),
}

#[derive(Debug, Clone)]
pub enum CompileOptionMsg {
    SetCompiler(String), ToggleShellEscape, ToggleBibtex, SetPasses(u8),
}

pub enum Modal { None, NewFile { name: String }, Settings, WelcomeScreen }

#[derive(Clone, Debug)]
pub enum StatusKind { Info, Success, Error, Warning }

//  State 

pub struct Ferroleaf {
    project: Option<Project>,
    file_tree: FileTree,
    editor_states: std::collections::HashMap<PathBuf, EditorState>,
    pdf_viewer: PdfViewer,
    compile_status: CompileStatus,
    compile_options: CompileOptions,
    compile_log: String,
    sidebar_visible: bool,
    log_panel_visible: bool,
    split_ratio: f32,
    font_size: u16,   // u16 — Pixels accepts From<u16>
    search_query: String,
    modal: Modal,
    status_message: Option<(String, StatusKind)>,
    latex_available: bool,
    available_compilers: Vec<CompilerKind>,
    /// Last known cursor position in window coordinates (updated via subscription).
    mouse_pos: (f32, f32),
    /// Currently open menu-bar dropdown (None = all closed).
    open_menu: Option<MenuKind>,
    /// Active right-click context menu.
    context_menu: Option<ContextMenuState>,
    /// Approximate window dimensions — used for context-menu clamping.
    window_width: f32,
    window_height: f32,
    /// Guard against opening a second native dialog while one is already open.
    dialog_open: bool,
    /// When true, the next PdfPagesRendered will auto-fit zoom and clear this flag.
    pdf_fit_on_next_render: bool,
    /// Accumulated vertical scroll of the editor pane (pixels from top).
    /// Updated from WheelScrolled events and used to sync the line-number gutter.
    editor_scroll_y: f32,
}

impl Ferroleaf {
    pub fn new() -> (Self, Task<Message>) {
        let latex_available     = Compiler::is_latex_available();
        let available_compilers = Compiler::available_compilers();
        let warn = if !latex_available {
            Some(("LaTeX not found - install texlive-full".into(), StatusKind::Warning))
        } else { None };
        let state = Ferroleaf {
            project: None, file_tree: Default::default(),
            editor_states: Default::default(), pdf_viewer: PdfViewer::new(),
            compile_status: CompileStatus::Idle,
            compile_options: CompileOptions::default(), compile_log: String::new(),
            sidebar_visible: true, log_panel_visible: false,
            split_ratio: 0.50, font_size: 14,
            search_query: String::new(), modal: Modal::WelcomeScreen,
            status_message: warn, latex_available, available_compilers,
            mouse_pos: (0.0, 0.0),
            open_menu: None, context_menu: None,
            window_width: 1440.0, window_height: 900.0,
            dialog_open: false,
            pdf_fit_on_next_render: false,
            editor_scroll_y: 0.0,
        };
        // Query the actual window size immediately so zoom-fit works correctly
        // even if the window is rendered at a different size than the default.
        let size_task = iced::window::get_oldest()
            .and_then(iced::window::get_size)
            .map(|size| Message::WindowResized(size.width, size.height));
        (state, size_task)
    }

    //  Update 
    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            // Use native Linux dialog tools (zenity / kdialog / yad / python3-gi).
            // These are spawned as child processes — no GTK main-loop needed,
            // works on both X11 and Wayland from any thread.
            Message::OpenProject => {
                if self.dialog_open { return Task::none(); }
                self.dialog_open = true;
                return Task::perform(
                    crate::dialog::pick_folder("Open LaTeX Project Folder"),
                    Message::ProjectOpened,
                );
            }

            Message::NewProject => {
                if self.dialog_open { return Task::none(); }
                self.dialog_open = true;
                return Task::perform(
                    crate::dialog::pick_folder("New Project - Choose Folder"),
                    |opt_path| match opt_path {
                        Some(path) => {
                            let main = path.join("main.tex");
                            if !main.exists() { let _ = std::fs::write(&main, TEMPLATE_ARTICLE); }
                            Message::ProjectOpened(Some(path))
                        }
                        None => Message::ProjectOpened(None),
                    },
                );
            }

            Message::ProjectOpened(Some(path)) => {
                self.dialog_open = false;
                let mut project = Project::new(path.clone());
                let settings = ProjectSettings::load(&project.root);
                if let Some(ref mf) = settings.main_file {
                    let c = project.root.join(mf);
                    if c.exists() { project.main_file = Some(c); }
                }
                self.compile_options.compiler = CompilerKind::from_str(&settings.compiler);
                self.compile_options.bibtex = settings.bibtex;
                self.compile_options.shell_escape = settings.shell_escape;
                let mf = project.main_file.clone();
                // Auto-expand the root directory so files are visible immediately
                self.file_tree.expanded_dirs.insert(path.clone());
                self.project = Some(project);
                self.modal = Modal::None;
                self.set_status(format!("Opened: {}", path.display()), StatusKind::Success);
                if let Some(f) = mf { return self.update(Message::OpenFile(f)); }
            }
            Message::ProjectOpened(None) => {
                self.dialog_open = false;
                self.set_status("No folder selected".into(), StatusKind::Info);
            }

            Message::OpenFile(path) => {
                if let Some(project) = &mut self.project {
                    if let Err(e) = project.open_file(path.clone()) {
                        self.set_status(format!("Open error: {}", e), StatusKind::Error);
                        return Task::none();
                    }
                    if !self.editor_states.contains_key(&path) {
                        if let Some(content) = project.get_content(&path) {
                            self.editor_states.insert(path.clone(), EditorState::new(path.clone(), content));
                        }
                    }
                }
            }
            Message::CloseTab(path) => {
                if let Some(p) = &mut self.project {
                    if p.is_dirty(&path) { let _ = p.save_file(&path); }
                    p.close_file(&path); self.editor_states.remove(&path);
                }
            }
            Message::SwitchTab(path) => {
                if let Some(p) = &mut self.project { p.active_file = Some(path); }
                self.editor_scroll_y = 0.0;
                return scrollable::scroll_to(
                    crate::editor::gutter_scroll_id(),
                    scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
                );
            }
            Message::ToggleFileTreeDir(path) => { self.file_tree.toggle_dir(&path); }
            Message::NewFile => { self.modal = Modal::NewFile { name: String::new() }; }
            Message::NewFileNameChanged(n) => {
                if let Modal::NewFile { name } = &mut self.modal { *name = n; }
            }
            Message::NewFileConfirmed => {
                if let Modal::NewFile { name } = &self.modal {
                    let n = name.clone();
                    if let Some(p) = &mut self.project {
                        match p.create_tex_file(&n) {
                            Ok(path) => { self.modal = Modal::None; return self.update(Message::OpenFile(path)); }
                            Err(e) => self.set_status(format!("Create failed: {}", e), StatusKind::Error),
                        }
                    }
                }
                self.modal = Modal::None;
            }
            Message::NewFileCancelled => { self.modal = Modal::None; }
            Message::SetMainFile(path) => {
                // Extract all needed data before any mutable borrow of self
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_string();

                if let Some(p) = &mut self.project {
                    p.set_main_file(path);
                }

                // Build and save settings (borrows self.project immutably now)
                if let Some(p) = &self.project {
                    let settings = crate::project::ProjectSettings {
                        main_file: p.main_file.as_ref()
                            .and_then(|f| f.file_name())
                            .and_then(|n| n.to_str())
                            .map(String::from),
                        compiler: self.compile_options.compiler.binary().to_string(),
                        bibtex: self.compile_options.bibtex,
                        shell_escape: self.compile_options.shell_escape,
                        extra_args: self.compile_options.extra_args.clone(),
                    };
                    let _ = settings.save(&p.root);
                }

                self.set_status(
                    format!("Main file set to: {}  ([M] in file tree)", name),
                    StatusKind::Success,
                );
            }
            Message::DeleteFile(path) => {
                if std::fs::remove_file(&path).is_ok() {
                    if let Some(p) = &mut self.project { p.close_file(&path); }
                    self.editor_states.remove(&path);
                }
            }

            Message::EditorAction(action) => {
                if let Some(project) = &mut self.project {
                    if let Some(active) = project.active_file.clone() {
                        if let Some(state) = self.editor_states.get_mut(&active) {
                            let is_edit = action.is_edit();
                            state.content.perform(action);
                            if is_edit { let t = state.text(); project.update_content(&active, t); }
                        }
                    }
                }
            }
            Message::EditorWheelScrolled(delta) => {
                // Only sync gutter when the mouse is inside the editor pane.
                let (mx, _my) = self.mouse_pos;
                let sidebar_w  = if self.sidebar_visible { 220.0_f32 } else { 0.0 };
                let editor_end = sidebar_w
                    + (self.window_width - sidebar_w) * self.split_ratio;
                if mx >= sidebar_w && mx <= editor_end {
                    // Convert the wheel delta to pixels using the same line height
                    // the gutter uses (font_size * 1.3), with 3 lines per tick.
                    let line_h = self.font_size as f32 * 1.3;
                    let dy = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => -y * 3.0 * line_h,
                        mouse::ScrollDelta::Pixels { y, .. } => -y,
                    };
                    self.editor_scroll_y = (self.editor_scroll_y + dy).max(0.0);
                    return scrollable::scroll_to(
                        crate::editor::gutter_scroll_id(),
                        scrollable::AbsoluteOffset { x: 0.0, y: self.editor_scroll_y },
                    );
                }
            }
            Message::SaveFile => {
                if let Some(project) = &mut self.project {
                    if let Some(active) = project.active_file.clone() {
                        if let Some(s) = self.editor_states.get(&active) {
                            let t = s.text(); project.update_content(&active, t);
                        }
                        match project.save_file(&active) {
                            Ok(_) => self.set_status("Saved".into(), StatusKind::Success),
                            Err(e) => self.set_status(format!("Save error: {}", e), StatusKind::Error),
                        }
                    }
                }
            }
            Message::SaveAll => {
                if let Some(project) = &mut self.project {
                    let paths: Vec<PathBuf> = self.editor_states.keys().cloned().collect();
                    for path in &paths {
                        if let Some(s) = self.editor_states.get(path) {
                            let t = s.text(); project.update_content(path, t);
                        }
                    }
                    match project.save_all() {
                        Ok(_) => self.set_status("All saved".into(), StatusKind::Success),
                        Err(e) => self.set_status(format!("Save error: {}", e), StatusKind::Error),
                    }
                }
            }
            Message::SearchChanged(q) => self.search_query = q,

            Message::Compile => {
                if !self.latex_available {
                    self.set_status("LaTeX not found - install texlive-full".into(), StatusKind::Error);
                    return Task::none();
                }
                let Some(project) = &self.project else { return Task::none(); };
                let Some(target) = project.compile_target().cloned() else {
                    self.set_status(
                        "No compile target -- open a .tex file, or click [M] next to a file to set it as main".into(),
                        StatusKind::Warning,
                    );
                    return Task::none();
                };
                let save = self.update(Message::SaveAll);
                self.compile_status = CompileStatus::Compiling { pass: 1, total: 1 };
                self.compile_log.clear(); self.log_panel_visible = true;
                let opts = self.compile_options.clone();
                let compile = Task::perform(async move {
                    let (tx, _rx) = mpsc::channel::<CompileStatus>(4);
                    let r = Compiler::compile(&target, &opts, tx).await;
                    Arc::new(if r.success { CompileStatus::Success(r) } else { CompileStatus::Failed(r) })
                }, Message::CompileStatusUpdate);
                return Task::batch(vec![save, compile]);
            }
            Message::CompileStatusUpdate(status) => {
                match status.as_ref() {
                    CompileStatus::Success(r) => {
                        self.compile_log = r.raw_log.clone();
                        let ne = r.errors().len(); let nw = r.warnings().len();
                        if let Some(pdf_path) = &r.pdf_path {
                            let path = pdf_path.clone();
                            self.compile_status = CompileStatus::Success(r.clone());
                            self.set_status(
                                format!("Done in {:.1}s : {} err, {} warn", r.duration.as_secs_f32(), ne, nw),
                                if ne == 0 { StatusKind::Success } else { StatusKind::Warning },
                            );
                            self.pdf_viewer.load_pdf(path.clone());
                            // Request a fit-to-width on the very next render.
                            // This flag is checked (and cleared) in PdfPagesRendered.
                            self.pdf_fit_on_next_render = true;
                            return Task::perform(async move {
                                // Render at zoom=1.0; PdfPagesRendered will rerender at fit zoom.
                                crate::pdf_viewer::render_pdf_pages(&path, 1.0).await.unwrap_or_default()
                            }, Message::PdfPagesRendered);
                        } else {
                            self.compile_status = CompileStatus::Failed(r.clone());
                            self.set_status(format!("Failed: {} errors", ne), StatusKind::Error);
                        }
                    }
                    CompileStatus::Failed(r) => {
                        self.compile_log = r.raw_log.clone();
                        self.compile_status = CompileStatus::Failed(r.clone());
                        self.set_status(format!("Failed: {} errors", r.errors().len()), StatusKind::Error);
                    }
                    other => { self.compile_status = other.clone(); }
                }
            }
            Message::CompileOptionChanged(m) => match m {
                CompileOptionMsg::SetCompiler(c)    => { self.compile_options.compiler = CompilerKind::from_str(&c); }
                CompileOptionMsg::ToggleShellEscape => { self.compile_options.shell_escape = !self.compile_options.shell_escape; }
                CompileOptionMsg::ToggleBibtex      => { self.compile_options.bibtex = !self.compile_options.bibtex; }
                CompileOptionMsg::SetPasses(n)      => { self.compile_options.passes = n.clamp(1, 3); }
            },
            Message::ClearLog => self.compile_log.clear(),

            Message::PdfPagesRendered(pages) => {
                // Always a full render now (all pages). Store and optionally auto-fit.
                self.pdf_viewer.total_pages = pages.len();
                // Capture unscaled page width once, from the initial zoom=1.0 render.
                if self.pdf_fit_on_next_render {
                    if let Some(first) = pages.first() {
                        self.pdf_viewer.base_page_width = Some(first.width);
                    }
                }
                self.pdf_viewer.rendered_pages = pages;

                if self.pdf_fit_on_next_render {
                    self.pdf_fit_on_next_render = false;
                    let sidebar_w   = if self.sidebar_visible { 220.0_f32 } else { 0.0 };
                    let pdf_panel_w = (self.window_width - sidebar_w)
                        * (1.0 - self.split_ratio) - 48.0;
                    let base_page_w = self.pdf_viewer.base_page_width
                        .map(|w| w as f32).unwrap_or(794.0);
                    if base_page_w > 0.0 && pdf_panel_w > 0.0 {
                        let fit_zoom = (pdf_panel_w / base_page_w).clamp(0.25, 4.0);
                        self.pdf_viewer.set_zoom(fit_zoom);
                        // Re-render at fit zoom. Flag is false so no loop.
                        return self.rerender_pdf();
                    }
                }
            }
            Message::MouseMoved(mx, my) => {
                self.mouse_pos = (mx, my);
            }

            Message::PdfClicked { page, x: _, y: _, page_w, page_h } => {
                // Use the real last-known cursor position instead of the sentinel.
                let (mx, my) = self.mouse_pos;
                if let Some(project) = &self.project {
                    if let Some(target) = project.compile_target() {
                        let pdf = target.with_extension("pdf");
                        if pdf.exists() {
                            return Task::perform(async move {
                                tokio::task::spawn_blocking(move || {
                                    synctex::pdf_to_source(&pdf, page as u32 + 1, mx, my, page_w, page_h)
                                }).await.ok().flatten()
                            }, Message::SynctexResult);
                        }
                    }
                }
            }
            Message::PdfNextPage => {
                self.pdf_viewer.next_page();
                let offset = self.pdf_viewer.scroll_offset_for_page(self.pdf_viewer.current_page);
                return scrollable::scroll_to(
                    crate::pdf_viewer::pdf_scroll_id(),
                    scrollable::AbsoluteOffset { x: 0.0, y: offset },
                );
            }
            Message::PdfPrevPage => {
                self.pdf_viewer.prev_page();
                let offset = self.pdf_viewer.scroll_offset_for_page(self.pdf_viewer.current_page);
                return scrollable::scroll_to(
                    crate::pdf_viewer::pdf_scroll_id(),
                    scrollable::AbsoluteOffset { x: 0.0, y: offset },
                );
            }
            Message::PdfZoomIn  => { self.pdf_viewer.zoom_in();  return self.rerender_pdf(); }
            Message::PdfZoomOut => { self.pdf_viewer.zoom_out(); return self.rerender_pdf(); }
            Message::PdfZoomFit  => {
                let sidebar_w   = if self.sidebar_visible { 220.0_f32 } else { 0.0 };
                let pdf_panel_w = (self.window_width - sidebar_w)
                    * (1.0 - self.split_ratio)
                    - 48.0;
                // Use the stable unscaled page width captured at zoom=1.0.
                // Never derive from current rendered width (which varies with zoom).
                let base_page_w = self.pdf_viewer.base_page_width
                    .map(|w| w as f32)
                    .unwrap_or(794.0);
                if base_page_w > 0.0 && pdf_panel_w > 0.0 {
                    let fit_zoom = (pdf_panel_w / base_page_w).clamp(0.25, 4.0);
                    self.pdf_viewer.set_zoom(fit_zoom);
                    return self.rerender_pdf();
                }
            }

            Message::SynctexResult(Some(loc)) => {
                let path = loc.file.clone(); let line = loc.line;
                let a = self.update(Message::OpenFile(path));
                let b = self.update(Message::GoToLine(line));
                return Task::batch(vec![a, b]);
            }
            Message::SynctexResult(None) => {
                self.set_status("SyncTeX: no match at this position".into(), StatusKind::Info);
            }
            Message::GoToLine(line) => {
                if let Some(project) = &self.project {
                    if let Some(active) = &project.active_file {
                        if let Some(s) = self.editor_states.get_mut(active) { s.jump_to_line(line); }
                    }
                }
                self.set_status(format!("Line {}", line), StatusKind::Info);
            }

            Message::WindowResized(w, h) => {
                self.window_width  = w;
                self.window_height = h;
            }

            Message::ToggleSidebar  => self.sidebar_visible  = !self.sidebar_visible,
            Message::ToggleLogPanel => self.log_panel_visible = !self.log_panel_visible,
            Message::ShowSettings   => self.modal = Modal::Settings,
            Message::CloseSettings  => {
                if let Some(p) = &self.project {
                    let s = ProjectSettings {
                        main_file: p.main_file.as_ref().and_then(|f| {
                            f.file_name().and_then(|n| n.to_str()).map(String::from)
                        }),
                        compiler: self.compile_options.compiler.binary().to_string(),
                        bibtex: self.compile_options.bibtex,
                        shell_escape: self.compile_options.shell_escape,
                        extra_args: self.compile_options.extra_args.clone(),
                    };
                    let _ = s.save(&p.root);
                }
                self.modal = Modal::None;
            }

            Message::KeyPressed(key, mods) => {
                if mods.command() {
                    match key.as_ref() {
                        keyboard::Key::Character("s") if mods.shift() => { return self.update(Message::SaveAll); }
                        keyboard::Key::Character("s")  => { return self.update(Message::SaveFile); }
                        keyboard::Key::Character("b")  => { return self.update(Message::Compile); }
                        keyboard::Key::Character("\\") => self.sidebar_visible = !self.sidebar_visible,
                        keyboard::Key::Character("n")  => { return self.update(Message::NewFile); }
                        keyboard::Key::Character("o")  => { return self.update(Message::OpenProject); }
                        keyboard::Key::Character("=") | keyboard::Key::Character("+") =>
                            self.font_size = (self.font_size + 1).min(32),
                        keyboard::Key::Character("-") =>
                            self.font_size = self.font_size.saturating_sub(1).max(8),
                        _ => {}
                    }
                }
            }
            //  Menu bar 
            Message::ToggleMenu(kind) => {
                if self.open_menu.as_ref() == Some(&kind) {
                    self.open_menu = None;
                } else {
                    self.open_menu = Some(kind);
                    self.context_menu = None;
                }
            }
            Message::Dismiss => {
                self.open_menu = None;
                self.context_menu = None;
            }

            //  Right-click context menu 
            Message::RightClicked => {
                let (mx, my) = self.mouse_pos;
                // Compute the pixel X where the editor panel ends.
                // Use a 20px inward safety margin so clicks near the divider
                // always get the editor context menu, never the PDF one.
                let sidebar_w  = if self.sidebar_visible { 220.0_f32 } else { 0.0 };
                let available  = self.window_width - sidebar_w;
                let editor_end = sidebar_w + available * self.split_ratio - 20.0;
                // Show PDF context menu only when cursor is clearly inside the
                // PDF pane AND a PDF has been rendered.
                let kind = if mx > editor_end
                    && self.pdf_viewer.pdf_path.is_some()
                    && !self.pdf_viewer.rendered_pages.is_empty()
                {
                    ContextMenuKind::PdfViewer
                } else {
                    ContextMenuKind::Editor
                };
                let (pw, ph) = self.pdf_viewer.rendered_pages
                    .get(self.pdf_viewer.current_page)
                    .map(|p| (p.width as f32, p.height as f32))
                    .unwrap_or((595.0, 842.0));
                self.context_menu = Some(ContextMenuState {
                    kind, x: mx, y: my, page_w: pw, page_h: ph,
                });
                self.open_menu = None;
            }

            //  SyncTeX triggered from context menu (uses saved click coords) 
            Message::SynctexAt { page, x, y, page_w, page_h } => {
                if let Some(project) = &self.project {
                    if let Some(target) = project.compile_target() {
                        let pdf = target.with_extension("pdf");
                        if pdf.exists() {
                            return Task::perform(async move {
                                tokio::task::spawn_blocking(move || {
                                    synctex::pdf_to_source(
                                        &pdf, page as u32 + 1, x, y, page_w, page_h,
                                    )
                                }).await.ok().flatten()
                            }, Message::SynctexResult);
                        }
                    }
                }
            }

            //  Editor helpers 
            Message::CommentLine => {
                let active_path = self.project.as_ref()
                    .and_then(|p| p.active_file.clone());
                if let Some(active) = active_path {
                    if let Some(state) = self.editor_states.get_mut(&active) {
                        let (line_idx, _) = state.cursor_position();
                        let src = state.text();
                        if let Some(line_content) = src.lines().nth(line_idx) {
                            state.content.perform(
                                text_editor::Action::Move(text_editor::Motion::Home)
                            );
                            if line_content.starts_with('%') {
                                state.content.perform(
                                    text_editor::Action::Edit(text_editor::Edit::Delete)
                                );
                            } else {
                                state.content.perform(
                                    text_editor::Action::Edit(text_editor::Edit::Insert('%'))
                                );
                            }
                            let new_text = state.text();
                            if let Some(p) = &mut self.project {
                                p.update_content(&active, new_text);
                            }
                        }
                    }
                }
            }
            Message::SelectAll => {
                if let Some(project) = &self.project {
                    if let Some(active) = project.active_file.clone() {
                        if let Some(state) = self.editor_states.get_mut(&active) {
                            state.content.perform(text_editor::Action::SelectAll);
                        }
                    }
                }
            }
            Message::CloseActiveTab => {
                if let Some(project) = &self.project {
                    if let Some(active) = project.active_file.clone() {
                        return self.update(Message::CloseTab(active));
                    }
                }
            }

            //  App control 
            Message::Quit => { std::process::exit(0); }
            Message::About => {
                self.set_status(
                    "Ferroleaf - Native LaTeX editor for Linux (iced 0.13 + SyncTeX)".into(),
                    StatusKind::Info,
                );
            }
            Message::ShowKeyboardShortcuts => { self.modal = Modal::WelcomeScreen; }

            Message::Noop => {}
            Message::Error(e) => self.set_status(e, StatusKind::Error),
        }
        Task::none()
    }

    //  View 
    pub fn view(&self) -> Element<Message> {
        let root: Element<Message> = container(
            column![
                self.view_toolbar(),
                self.view_menu_bar(),
                self.view_main(),
                self.view_status(),
            ].spacing(0).width(Length::Fill).height(Length::Fill)
        )
        .width(Length::Fill).height(Length::Fill)
        .style(crate::theme::sidebar)
        .into();

        // Layer dropdown / context-menu overlays on top via stack!
        let mut layers = stack![root];
        if self.open_menu.is_some() || self.context_menu.is_some() {
            // Transparent full-window click-catcher — sits below the menu.
            layers = layers.push(dismiss_catcher());
        }
        if let Some(menu) = &self.open_menu {
            layers = layers.push(self.view_menu_dropdown(menu));
        }
        if let Some(ctx) = &self.context_menu {
            layers = layers.push(self.view_context_menu_overlay(ctx));
        }
        let layered: Element<Message> = layers.into();

        match &self.modal {
            Modal::None             => layered,
            Modal::WelcomeScreen    => overlay_welcome(layered),
            Modal::NewFile { name } => overlay_new_file(layered, name),
            Modal::Settings         => self.overlay_settings(layered),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::event::listen_with(|ev, _status, _id| match ev {
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) =>
                Some(Message::KeyPressed(key, modifiers)),
            Event::Mouse(mouse::Event::CursorMoved { position }) =>
                Some(Message::MouseMoved(position.x, position.y)),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) =>
                Some(Message::RightClicked),
            Event::Mouse(mouse::Event::WheelScrolled { delta }) =>
                Some(Message::EditorWheelScrolled(delta)),
            Event::Window(iced::window::Event::Resized(size)) =>
                Some(Message::WindowResized(size.width, size.height)),
            _ => None,
        })
    }

    pub fn theme(&self) -> Theme { Theme::Dark }

    //  View helpers 

    fn view_toolbar(&self) -> Element<Message> {
        let compiling = self.compile_status.is_running();
        let compile_icon = if compiling { crate::icons::COMPILING } else { crate::icons::COMPILE };
        let compile_label = if compiling { " Compiling..." } else { " Compile" };

        container(row![
            container(text("Ferroleaf").size(14).color(Palette::PINK_BRIGHT))
                .padding([0u16, 14u16]),
            tip(svg_btn(crate::icons::SIDEBAR,     Message::ToggleSidebar,  20), "Toggle Sidebar  Ctrl+\\"),
            tip(svg_btn(crate::icons::OPEN_FOLDER, Message::OpenProject,    20), "Open Project Folder  Ctrl+O"),
            tip(svg_btn(crate::icons::NEW_FILE,    Message::NewFile,        20), "New File  Ctrl+N"),
            tip(svg_btn(crate::icons::SETTINGS,    Message::ShowSettings,   20), "Settings"),
            Space::with_width(Length::Fill),
            tip(svg_btn(crate::icons::LOG,         Message::ToggleLogPanel, 20), "Toggle Compiler Log"),
            button(
                row![
                    svg(svg::Handle::from_memory(compile_icon))
                        .width(16).height(16),
                    text(compile_label).size(13),
                ].spacing(6).align_y(Alignment::Center)
            )
            .on_press(Message::Compile)
            .style(crate::theme::primary_button)
            .padding([5u16, 14u16]),
            Space::with_width(8),
        ].spacing(4).align_y(Alignment::Center).height(42))
        .width(Length::Fill)
        .style(crate::theme::toolbar)
        .into()
    }

    fn view_main(&self) -> Element<Message> {
        let ew = (self.split_ratio * 100.0) as u16;
        let pw = ((1.0 - self.split_ratio) * 100.0) as u16;

        let center: Element<Message> = row![
            container(self.view_editor()).width(Length::FillPortion(ew)).height(Length::Fill),
            container(self.view_pdf()).width(Length::FillPortion(pw)).height(Length::Fill),
        ].width(Length::Fill).height(Length::Fill).into();

        let with_sidebar: Element<Message> = if self.sidebar_visible {
            row![self.view_sidebar(), center]
                .width(Length::Fill).height(Length::Fill).into()
        } else { center };

        if self.log_panel_visible {
            column![
                with_sidebar,
                container(self.view_log())
                    .width(Length::Fill)
                    .height(Length::FillPortion(1)),
            ]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        } else { with_sidebar }
    }

    fn view_sidebar(&self) -> Element<Message> {
        let entries: Vec<crate::project::FileEntry> =
            self.project.as_ref().map(|p| p.visible_files()).unwrap_or_default();
        let active = self.project.as_ref().and_then(|p| p.active_file.as_ref());

        let main_file = self.project.as_ref().and_then(|p| p.main_file.as_ref());
        // entries is moved into file_tree.view — no borrow escapes
        let tree = self.file_tree.view(entries, active, main_file, &self.search_query);

        container(column![
            row![
                text("Files").size(12).color(Palette::TEXT_DIM),
                Space::with_width(Length::Fill),
                tip(ib("+", Message::NewFile), "New File  Ctrl+N"),
            ].align_y(Alignment::Center).padding([6u16, 8u16]),
            container(
                text_input("Search...", &self.search_query)
                    .on_input(Message::SearchChanged)
                    .size(12).padding([4u16, 8u16])
                    .style(crate::theme::search_input)
            ).padding([0u16, 6u16]),
            horizontal_rule(1).style(crate::theme::subtle_rule),
            tree,
        ].spacing(0).width(Length::Fill).height(Length::Fill))
        .width(220).height(Length::Fill)
        .style(crate::theme::sidebar)
        .into()
    }

    fn view_editor(&self) -> Element<Message> {
        let Some(project) = &self.project else {
            return blank("Open a project to start editing");
        };
        let tabs = tab_bar(
            &project.open_files,
            project.active_file.as_ref(),
            |p| project.is_dirty(p),
        );
        let font_size = self.font_size;
        let body: Element<Message> = if let Some(active) = &project.active_file {
            if let Some(state) = self.editor_states.get(active) {
                let lc = state.text().lines().count();
                container(row![
                    crate::editor::line_gutter(lc, font_size),
                    container(
                        text_editor(&state.content)
                            .on_action(Message::EditorAction)
                            .font(Font::MONOSPACE)
                            .size(font_size)
                            .style(crate::theme::code_editor)
                            .highlight_with::<LatexHighlighter>(
                                LatexHighlightSettings,
                                latex_highlight_format,
                            )
                            .height(Length::Fill)
                    ).width(Length::Fill).padding([4u16, 4u16]),
                ].spacing(0))
                .width(Length::Fill).height(Length::Fill)
                .style(crate::theme::editor_pane)
                .into()
            } else { blank("Loading...") }
        } else { blank("Select a file to edit") };

        column![tabs, body].spacing(0).width(Length::Fill).height(Length::Fill).into()
    }

    fn view_pdf(&self) -> Element<Message> { self.pdf_viewer.view() }

    //  Menu bar 

    fn view_menu_bar(&self) -> Element<Message> {
        let mk = |label: &'static str, kind: MenuKind| -> Element<'static, Message> {
            let active = self.open_menu.as_ref() == Some(&kind);
            button(text(label).size(12u16))
                .on_press(Message::ToggleMenu(kind))
                .style(if active {
                    crate::theme::menu_bar_active
                } else {
                    crate::theme::menu_bar_item
                })
                .padding([4u16, 12u16])
                .into()
        };
        container(row![
            mk("File",  MenuKind::File),
            mk("Edit",  MenuKind::Edit),
            mk("Build", MenuKind::Build),
            mk("View",  MenuKind::View),
            mk("Help",  MenuKind::Help),
            Space::with_width(Length::Fill),
        ].spacing(0).align_y(Alignment::Center).height(28))
        .width(Length::Fill)
        .style(crate::theme::menu_bar_bg)
        .into()
    }

    /// Render the open dropdown under the menu bar.
    fn view_menu_dropdown(&self, menu: &MenuKind) -> Element<'_, Message> {
        // Approximate left offset (px) for each menu button from window left edge.
        let left: f32 = match menu {
            MenuKind::File  =>   0.0,
            MenuKind::Edit  =>  50.0,
            MenuKind::Build => 100.0,
            MenuKind::View  => 160.0,
            MenuKind::Help  => 212.0,
        };

        let items: Vec<Element<Message>> = match menu {
            MenuKind::File => vec![
                dmi("New Project",           Message::NewProject),
                dmi("Open Project  Ctrl+O",  Message::OpenProject),
                dms(),
                dmi("New File      Ctrl+N",  Message::NewFile),
                dmi("Save          Ctrl+S",  Message::SaveFile),
                dmi("Save All      Ctrl+S+S",Message::SaveAll),
                dmi("Close Tab",             Message::CloseActiveTab),
                dms(),
                dmi("Settings",              Message::ShowSettings),
                dms(),
                dmi("Quit",                  Message::Quit),
            ],
            MenuKind::Edit => vec![
                dmi("Undo         Ctrl+Z",   Message::Noop),
                dmi("Redo         Ctrl+Y",   Message::Noop),
                dms(),
                dmi("Cut          Ctrl+X",   Message::Noop),
                dmi("Copy         Ctrl+C",   Message::Noop),
                dmi("Paste        Ctrl+V",   Message::Noop),
                dmi("Select All   Ctrl+A",   Message::SelectAll),
                dms(),
                dmi("Comment / Uncomment Line", Message::CommentLine),
            ],
            MenuKind::Build => {
                let c = &self.compile_options.compiler;
                vec![
                    dmi("Compile  Ctrl+B", Message::Compile),
                    dms(),
                    dmtick("pdfLaTeX",  *c == CompilerKind::PdfLatex,
                        Message::CompileOptionChanged(CompileOptionMsg::SetCompiler("pdflatex".into()))),
                    dmtick("XeLaTeX",   *c == CompilerKind::XeLatex,
                        Message::CompileOptionChanged(CompileOptionMsg::SetCompiler("xelatex".into()))),
                    dmtick("LuaLaTeX",  *c == CompilerKind::LuaLatex,
                        Message::CompileOptionChanged(CompileOptionMsg::SetCompiler("lualatex".into()))),
                    dms(),
                    dmcheck("BibTeX",       self.compile_options.bibtex,
                        Message::CompileOptionChanged(CompileOptionMsg::ToggleBibtex)),
                    dmcheck("Shell Escape", self.compile_options.shell_escape,
                        Message::CompileOptionChanged(CompileOptionMsg::ToggleShellEscape)),
                ]
            },
            MenuKind::View => vec![
                dmcheck("Sidebar      Ctrl+\\", self.sidebar_visible,  Message::ToggleSidebar),
                dmcheck("Log Panel",             self.log_panel_visible, Message::ToggleLogPanel),
                dms(),
                dmi("Zoom In",    Message::PdfZoomIn),
                dmi("Zoom Out",   Message::PdfZoomOut),
                dmi("Fit Page",   Message::PdfZoomFit),
                dms(),
                dmi("Prev Page",  Message::PdfPrevPage),
                dmi("Next Page",  Message::PdfNextPage),
            ],
            MenuKind::Help => vec![
                dmi("Keyboard Shortcuts", Message::ShowKeyboardShortcuts),
                dms(),
                dmi("About Ferroleaf",    Message::About),
            ],
        };

        let dropdown = container(column(items).spacing(0).padding([4u16, 0u16]))
            .style(crate::theme::menu_dropdown_bg)
            .width(240);

        // Position using Space offsets inside a full-window container.
        container(
            column![
                Space::with_height(Length::Fixed(70.0)), // toolbar(42) + menu-bar(28)
                row![
                    Space::with_width(Length::Fixed(left)),
                    dropdown,
                ]
            ]
        )
        .width(Length::Fill).height(Length::Fill)
        .into()
    }

    /// Render the right-click context menu at the stored click position.
    fn view_context_menu_overlay(&self, ctx: &ContextMenuState) -> Element<'_, Message> {
        let items: Vec<Element<Message>> = match ctx.kind {
            ContextMenuKind::Editor => vec![
                dmi("Undo",                        Message::Noop),
                dmi("Redo",                        Message::Noop),
                dms(),
                dmi("Cut",                         Message::Noop),
                dmi("Copy",                        Message::Noop),
                dmi("Paste",                       Message::Noop),
                dmi("Select All",                  Message::SelectAll),
                dms(),
                dmi("Comment / Uncomment Line",    Message::CommentLine),
                dms(),
                dmi("Compile   Ctrl+B",            Message::Compile),
            ],
            ContextMenuKind::PdfViewer => {
                let page = self.pdf_viewer.current_page;
                let (pw, ph) = (ctx.page_w, ctx.page_h);
                vec![
                    dmi("Jump to Source (SyncTeX)", Message::SynctexAt {
                        page, x: ctx.x, y: ctx.y, page_w: pw, page_h: ph,
                    }),
                    dms(),
                    dmi("Zoom In",    Message::PdfZoomIn),
                    dmi("Zoom Out",   Message::PdfZoomOut),
                    dmi("Fit Page",   Message::PdfZoomFit),
                    dms(),
                    dmi("Prev Page",  Message::PdfPrevPage),
                    dmi("Next Page",  Message::PdfNextPage),
                    dms(),
                    dmi("Recompile",  Message::Compile),
                ]
            },
        };

        let menu = container(column(items).spacing(0).padding([4u16, 0u16]))
            .style(crate::theme::menu_dropdown_bg)
            .width(230);

        // Clamp so the menu never overflows the right / bottom edge.
        let cx = ctx.x.min(self.window_width  - 240.0).max(0.0);
        let cy = ctx.y.min(self.window_height - 360.0).max(0.0);

        container(
            column![
                Space::with_height(Length::Fixed(cy)),
                row![
                    Space::with_width(Length::Fixed(cx)),
                    menu,
                ]
            ]
        )
        .width(Length::Fill).height(Length::Fill)
        .into()
    }

    fn view_log(&self) -> Element<Message> {
        let diags: Vec<Element<Message>> = self.compile_status.last_result().map(|r| {
            r.diagnostics.iter().map(|d| {
                let (pfx, col) = match d.level {
                    DiagnosticLevel::Error   => ("[E]", Palette::ERROR),
                    DiagnosticLevel::Warning => ("[W]", Palette::WARNING),
                    DiagnosticLevel::Info    => ("[i]", Palette::TEXT_DIM),
                };
                let loc = match (d.file.as_ref(), d.line) {
                    (Some(f), Some(l)) => format!("{}:{} - ", f, l),
                    (Some(f), None)    => format!("{} - ", f),
                    _                  => String::new(),
                };
                let lbl = row![
                    text(pfx).size(12).color(col),
                    text(format!("{}{}", loc, d.message)).size(12).color(Palette::TEXT_SECONDARY),
                ].spacing(6).padding([2u16, 8u16]);
                if let Some(line) = d.line {
                    button(lbl).on_press(Message::GoToLine(line))
                        .style(crate::theme::ghost_button)
                        .width(Length::Fill).padding(0u16).into()
                } else { lbl.into() }
            }).collect()
        }).unwrap_or_default();

        container(column![
            row![
                text("Compiler Log").size(12).color(Palette::TEXT_DIM),
                Space::with_width(Length::Fill),
                tip(ib("Clear", Message::ClearLog), "Clear compiler log"),
                tip(ib("X", Message::ToggleLogPanel), "Close log panel"),
            ].align_y(Alignment::Center).padding([4u16, 8u16]),
            horizontal_rule(1).style(crate::theme::subtle_rule),
            scrollable(column![
                column(diags).spacing(2),
                Space::with_height(4),
                text(&self.compile_log).size(11u16).font(Font::MONOSPACE).color(Palette::TEXT_DIM),
            ].padding([4u16, 8u16]))
            .height(Length::Fill)
            .style(crate::theme::log_scroll),
        ].spacing(0))
        .width(Length::Fill)
        .style(crate::theme::log_panel)
        .into()
    }

    fn view_status(&self) -> Element<Message> {
        let (msg_str, col) = self.status_message.as_ref().map(|(m, k)| {
            let c = match k {
                StatusKind::Success => Palette::SUCCESS,
                StatusKind::Error   => Palette::ERROR,
                StatusKind::Warning => Palette::WARNING,
                StatusKind::Info    => Palette::TEXT_SECONDARY,
            };
            (m.clone(), c)  // clone the String to avoid borrow of local
        }).unwrap_or_else(|| ("Ready".to_string(), Palette::TEXT_DIM));

        let pill = match &self.compile_status {
            CompileStatus::Idle                    => text("[ ]").size(11u16).color(Palette::TEXT_DIM),
            CompileStatus::Compiling { pass, total } =>
                text(format!("Building {}/{}", pass, total)).size(11u16).color(Palette::WARNING),
            CompileStatus::RunningBibtex => text("BibTeX...").size(11u16).color(Palette::WARNING),
            CompileStatus::Success(_)    => text("[OK]").size(11u16).color(Palette::SUCCESS),
            CompileStatus::Failed(_)     => text("[ERR]").size(11u16).color(Palette::ERROR),
        };

        let cursor = self.project.as_ref()
            .and_then(|p| p.active_file.as_ref())
            .and_then(|f| self.editor_states.get(f))
            .map(|s| { let (l, c) = s.cursor_position(); format!("Ln {}, Col {}", l + 1, c + 1) })
            .unwrap_or_default();

        let compiler_name = self.compile_options.compiler.display().to_string();
        // Show which file Ctrl+B will compile, and whether it's pinned (★) or active tab
        let target_name = self.project.as_ref().and_then(|p| {
            let is_active_tex = p.active_file.as_ref()
                .and_then(|f| f.extension().and_then(|e| e.to_str()))
                == Some("tex");
            let is_pinned = p.main_file.is_some();
            p.compile_target().and_then(|t| t.file_name()).and_then(|n| n.to_str())
                .map(|s| {
                    if is_active_tex {
                        format!(" -> {} (active tab)", s)
                    } else if is_pinned {
                        format!(" -> {} (main)", s)
                    } else {
                        format!(" -> {}", s)
                    }
                })
        }).unwrap_or_default();

        container(row![
            Space::with_width(8),
            text(msg_str).size(11u16).color(col),
            Space::with_width(Length::Fill),
            text(cursor).size(11u16).color(Palette::TEXT_DIM),
            Space::with_width(12),
            text(format!("{}{}", compiler_name, target_name)).size(11u16).color(Palette::TEXT_DIM),
            Space::with_width(12),
            pill,
            Space::with_width(8),
        ].align_y(Alignment::Center).height(24))
        .width(Length::Fill)
        .style(crate::theme::status_bar)
        .into()
    }

    fn overlay_settings<'a>(&'a self, base: Element<'a, Message>) -> Element<'a, Message> {
        // Explicit fn pointer style to avoid `as _` inference failures
        type BtnStyle = fn(&Theme, button::Status) -> button::Style;
        let primary: BtnStyle = crate::theme::primary_button;
        let ghost:   BtnStyle = crate::theme::ghost_button;

        let cbtns: Vec<Element<Message>> = self.available_compilers.iter().map(|c| {
            let active = c.binary() == self.compile_options.compiler.binary();
            button(text(c.display()).size(13))
                .on_press(Message::CompileOptionChanged(
                    CompileOptionMsg::SetCompiler(c.binary().to_string())
                ))
                .style(if active { primary } else { ghost })
                .padding([5u16, 14u16])
                .into()
        }).collect();

        let pbtns: Vec<Element<Message>> = (1u8..=3).map(|n| {
            let active = n == self.compile_options.passes;
            button(text(format!("{}", n)).size(13))
                .on_press(Message::CompileOptionChanged(CompileOptionMsg::SetPasses(n)))
                .style(if active { primary } else { ghost })
                .padding([5u16, 14u16])
                .into()
        }).collect();

        let card = container(column![
            row![
                text("Settings").size(22).color(Palette::PINK_BRIGHT),
                Space::with_width(Length::Fill),
                ib("X", Message::CloseSettings),
            ].align_y(Alignment::Center),
            Space::with_height(16),
            text("Compiler").size(12).color(Palette::TEXT_DIM),
            row(cbtns).spacing(8),
            Space::with_height(12),
            text("Options").size(12).color(Palette::TEXT_DIM),
            tog("BibTeX",       self.compile_options.bibtex,
                Message::CompileOptionChanged(CompileOptionMsg::ToggleBibtex)),
            tog("Shell escape", self.compile_options.shell_escape,
                Message::CompileOptionChanged(CompileOptionMsg::ToggleShellEscape)),
            Space::with_height(12),
            text("Compile passes").size(12).color(Palette::TEXT_DIM),
            row(pbtns).spacing(8),
            Space::with_height(20),
            button(text("Close & Save").size(13)).on_press(Message::CloseSettings)
                .style(crate::theme::primary_button).padding([8u16, 28u16]),
        ].spacing(10).align_x(Alignment::Start).padding([16u16, 32u16]).width(420))
        .style(crate::theme::card);

        stack![
            base,
            container(card).width(Length::Fill).height(Length::Fill)
                .center_x(Length::Fill).center_y(Length::Fill)
                .style(crate::theme::overlay),
        ].into()
    }

    fn set_status(&mut self, msg: String, kind: StatusKind) {
        self.status_message = Some((msg, kind));
    }

    /// Re-render all pages at the current zoom.
    fn rerender_pdf(&self) -> Task<Message> {
        if let Some(path) = &self.pdf_viewer.pdf_path {
            let path = path.clone(); let zoom = self.pdf_viewer.zoom;
            Task::perform(
                async move {
                    crate::pdf_viewer::render_pdf_pages(&path, zoom).await.unwrap_or_default()
                },
                Message::PdfPagesRendered,
            )
        } else { Task::none() }
    }

}

//  Free widget helpers 

fn ib(icon: &'static str, msg: Message) -> Element<'static, Message> {
    button(text(icon).size(18u16))
        .on_press(msg)
        .style(crate::theme::icon_button)
        .padding([4u16, 10u16])
        .into()
}

/// SVG icon button for the toolbar.
fn svg_btn(data: &'static [u8], msg: Message, size: u16) -> Element<'static, Message> {
    let handle = svg::Handle::from_memory(data);
    button(
        svg(handle)
            .width(Length::Fixed(size as f32))
            .height(Length::Fixed(size as f32))
    )
    .on_press(msg)
    .style(crate::theme::icon_button)
    .padding([5u16, 8u16])
    .into()
}

/// Wrap any element with a bottom tooltip using the dark tooltip style.
fn tip(content: Element<'static, Message>, label: &'static str) -> Element<'static, Message> {
    tooltip(
        content,
        container(text(label).size(11u16).color(crate::theme::Palette::TEXT_PRIMARY))
            .padding([4u16, 8u16])
            .style(crate::theme::tooltip_box),
        tooltip::Position::Bottom,
    )
    .into()
}

/// Menu / context-menu item button.
fn dmi(label: impl ToString, msg: Message) -> Element<'static, Message> {
    button(
        text(label.to_string()).size(13u16).color(Palette::TEXT_SECONDARY)
    )
    .on_press(msg)
    .style(crate::theme::menu_item)
    .width(Length::Fill)
    .padding([7u16, 16u16])
    .into()
}

/// Menu separator rule.
fn dms() -> Element<'static, Message> {
    container(horizontal_rule(1).style(crate::theme::subtle_rule))
        .padding([3u16, 8u16])
        .width(Length::Fill)
        .into()
}

/// Menu item with a checkmark toggle ([x]/[ ] prefix).
fn dmcheck(label: &str, checked: bool, msg: Message) -> Element<'static, Message> {
    let prefix = if checked { "[x] " } else { "[ ] " };
    let color  = if checked { Palette::PINK_BRIGHT } else { Palette::TEXT_SECONDARY };
    button(
        text(format!("{}{}", prefix, label)).size(13u16).color(color)
    )
    .on_press(msg)
    .style(crate::theme::menu_item)
    .width(Length::Fill)
    .padding([7u16, 16u16])
    .into()
}

/// Menu item with a radio-button indicator ([*]/[ ] prefix).
fn dmtick(label: &str, selected: bool, msg: Message) -> Element<'static, Message> {
    let prefix = if selected { "[*] " } else { "[ ] " };
    let color  = if selected { Palette::PINK_BRIGHT } else { Palette::TEXT_SECONDARY };
    button(
        text(format!("{}{}", prefix, label)).size(13u16).color(color)
    )
    .on_press(msg)
    .style(crate::theme::menu_item)
    .width(Length::Fill)
    .padding([7u16, 16u16])
    .into()
}

/// Invisible full-window button that dismisses any open menu/context-menu.
fn dismiss_catcher() -> Element<'static, Message> {
    button(
        container(Space::new(Length::Fill, Length::Fill))
            .width(Length::Fill).height(Length::Fill)
    )
    .on_press(Message::Dismiss)
    .style(|_, _| button::Style { background: None, ..Default::default() })
    .width(Length::Fill).height(Length::Fill)
    .into()
}

fn blank(label: &'static str) -> Element<'static, Message> {
    container(text(label).size(14u16).color(Palette::TEXT_DIM))
        .center_x(Length::Fill).center_y(Length::Fill)
        .width(Length::Fill).height(Length::Fill)
        .style(crate::theme::editor_pane)
        .into()
}

fn krow(k: &'static str, a: &'static str) -> Element<'static, Message> {
    row![
        container(text(k).size(11u16).font(Font::MONOSPACE).color(Palette::PINK_DIM)).width(165),
        text(a).size(11u16).color(Palette::TEXT_DIM),
    ].spacing(8).into()
}

fn tog(label: &'static str, on: bool, msg: Message) -> Element<'static, Message> {
    type BtnStyle = fn(&Theme, button::Status) -> button::Style;
    let primary: BtnStyle = crate::theme::primary_button;
    let ghost:   BtnStyle = crate::theme::ghost_button;
    row![
        text(label).size(13u16).color(Palette::TEXT_SECONDARY),
        Space::with_width(Length::Fill),
        button(text(if on { "ON " } else { "OFF" }).size(12u16))
            .on_press(msg)
            .style(if on { primary } else { ghost })
            .padding([4u16, 12u16]),
    ].align_y(Alignment::Center).into()
}

fn overlay_welcome(base: Element<'_, Message>) -> Element<'_, Message> {
    let card = container(column![
        text("# Ferroleaf").size(38u16).color(Palette::PINK_BRIGHT),
        text("Native LaTeX editor for Linux").size(13u16).color(Palette::TEXT_SECONDARY),
        Space::with_height(24),
        button(text("  Open Project Folder  ").size(14u16))
            .on_press(Message::OpenProject)
            .style(crate::theme::primary_button).padding([10u16, 28u16]),
        Space::with_height(6),
        button(text("  New Project  ").size(14u16))
            .on_press(Message::NewProject)
            .style(crate::theme::ghost_button).padding([8u16, 28u16]),
        Space::with_height(20),
        text("Keyboard shortcuts").size(11u16).color(Palette::TEXT_DIM),
        krow("Ctrl+B",       "Compile"),
        krow("Ctrl+S",       "Save file"),
        krow("Ctrl+Shift+S", "Save all"),
        krow("Ctrl+O",       "Open project"),
        krow("Ctrl+N",       "New file"),
        krow("Ctrl+\\",      "Toggle sidebar"),
        krow("Click on PDF", "Jump to LaTeX source (SyncTeX)"),
    ].spacing(8).align_x(Alignment::Center).padding([16u16, 44u16]))
    .style(crate::theme::card)
    .width(440);

    stack![
        base,
        container(card).width(Length::Fill).height(Length::Fill)
            .center_x(Length::Fill).center_y(Length::Fill)
            .style(crate::theme::overlay),
    ].into()
}

fn overlay_new_file<'a>(base: Element<'a, Message>, name: &'a str) -> Element<'a, Message> {
    let card = container(column![
        text("New File").size(22u16).color(Palette::PINK_BRIGHT),
        Space::with_height(12),
        text_input("filename.tex", name)
            .on_input(Message::NewFileNameChanged)
            .on_submit(Message::NewFileConfirmed)
            .size(14u16).padding([10u16, 10u16])
            .style(crate::theme::search_input),
        Space::with_height(16),
        row![
            button(text("Create").size(13u16)).on_press(Message::NewFileConfirmed)
                .style(crate::theme::primary_button).padding([8u16, 22u16]),
            button(text("Cancel").size(13u16)).on_press(Message::NewFileCancelled)
                .style(crate::theme::ghost_button).padding([8u16, 16u16]),
        ].spacing(10),
    ].spacing(10).align_x(Alignment::Start).padding([16u16, 32u16]).width(370))
    .style(crate::theme::card);

    stack![
        base,
        container(card).width(Length::Fill).height(Length::Fill)
            .center_x(Length::Fill).center_y(Length::Fill)
            .style(crate::theme::overlay),
    ].into()
}

const TEMPLATE_ARTICLE: &str = r#"\documentclass[12pt,a4paper]{article}
\usepackage[utf8]{inputenc}
\usepackage[T1]{fontenc}
\usepackage{amsmath,amssymb,amsthm}
\usepackage{graphicx}
\usepackage{hyperref}
\usepackage{geometry}
\geometry{margin=2.5cm}

\title{My Document}
\author{Author Name}
\date{\today}

\begin{document}
\maketitle
\begin{abstract}Your abstract goes here.\end{abstract}
\section{Introduction}Start writing here.
\section{Conclusion}Your conclusion.
\end{document}
"#;

# ⊛ Ferroleaf

A native Linux LaTeX editor built with **Rust + Iced**, featuring a warm **pink-brown dark theme**, real-time PDF preview, and full **SyncTeX** support — clicking any text in the PDF jumps your cursor directly to the corresponding LaTeX source.

No Electron. No browser. No subscription. All features, always.

---

## Features

| Feature | Details |
|---|---|
| **Native Iced UI** | Pure Rust, GPU-accelerated via wgpu — no Electron, no web runtime |
| **Pink-brown dark theme** | Custom warm palette: deep browns, rose pinks, high-contrast text |
| **PDF preview** | Side-by-side live PDF rendered via pdfium |
| **SyncTeX (PDF→source)** | Click any text in the PDF — cursor jumps to the matching `.tex` line |
| **Multi-file projects** | Full project workspace, compiles the **main file** regardless of which tab is open |
| **Intermediate file filtering** | `.aux`, `.log`, `.synctex.gz`, `.bbl` etc. are **never shown** in the file tree |
| **Multiple compilers** | pdfLaTeX, XeLaTeX, LuaLaTeX — auto-detected from `$PATH` |
| **BibTeX support** | Optional BibTeX pass between LaTeX runs |
| **Multi-pass compilation** | 1–3 LaTeX passes for TOC/refs/bibliography |
| **Syntax highlighting** | Commands, math, comments, braces, keywords in distinct colors |
| **Line numbers** | Always-visible gutter |
| **Compiler log panel** | Errors and warnings with clickable source lines |
| **Keyboard shortcuts** | Ctrl+B compile, Ctrl+S save, Ctrl+O open, etc. |
| **Shell escape** | Toggleable for TikZ-externalize and friends |
| **Project settings** | Persisted in `.ferroleaf.json` at project root |

---

## Quick Start

```bash
# Clone / extract the project
cd ferroleaf

# Build and install (handles dependencies automatically)
chmod +x install.sh
./install.sh

# Or build manually
cargo build --release
./target/release/ferroleaf
```

---

## Build Requirements

### Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### System libraries (Ubuntu/Debian)
```bash
sudo apt-get install -y \
    build-essential pkg-config \
    libssl-dev libgtk-3-dev \
    libx11-dev libxcb1-dev \
    libvulkan-dev \
    fonts-jetbrains-mono \
    texlive-full synctex
```

### System libraries (Fedora/RHEL)
```bash
sudo dnf install -y \
    gcc openssl-devel gtk3-devel \
    libX11-devel vulkan-loader-devel \
    jetbrains-mono-fonts \
    texlive-scheme-full
```

### pdfium (for PDF rendering)
The `install.sh` script downloads this automatically. If you prefer manual setup:

```bash
# Download from https://github.com/bblanchon/pdfium-binaries/releases
# Place libpdfium.so next to the ferroleaf binary
export PDFIUM_DYNAMIC_LIB_PATH=/path/to/pdfium/lib
```

Alternatively, pdfium is often available via system package managers:
```bash
# Ubuntu (unofficial PPA)
sudo add-apt-repository ppa:saiarcot895/chromium-dev
sudo apt-get install libpdfium-dev
```

---

## Usage

### Opening a project
1. Launch Ferroleaf — the welcome screen appears
2. Click **Open Project Folder** and select your LaTeX project directory
3. Ferroleaf auto-detects `main.tex` (or whichever `.tex` file contains `\documentclass`)

### Compiling
- Press **Ctrl+B** or click **▶ Compile** in the toolbar
- The PDF appears in the right panel when compilation succeeds
- Errors and warnings appear in the log panel at the bottom

### SyncTeX (click-to-source)
1. Compile your document first (generates `.synctex.gz`)
2. Click any text in the PDF viewer
3. The editor scrolls to the corresponding LaTeX source line

### Multi-file projects
- Ferroleaf compiles the **main file** (shown with ★ in the file tree), not the currently open tab
- Right-click any `.tex` file in the file tree to **Set as main file**
- Main file is remembered in `.ferroleaf.json`

### Intermediate files
The following are automatically hidden from the file tree:
`.aux .log .out .toc .lof .lot .bbl .blg .fls .fdb_latexmk .synctex.gz .dvi .idx .ilg .ind .bcf .run.xml .nav .snm .vrb`

---

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl+B` | Compile document |
| `Ctrl+S` | Save active file |
| `Ctrl+Shift+S` | Save all files |
| `Ctrl+O` | Open project folder |
| `Ctrl+N` | New file |
| `Ctrl+\` | Toggle file tree sidebar |
| `Ctrl+F` | Toggle find/replace panel |
| `Ctrl+=` / `Ctrl++` | Increase font size |
| `Ctrl+-` | Decrease font size |

---

## Project Structure

```
ferroleaf/
├── src/
│   ├── main.rs          # Entry point, Iced settings
│   ├── app.rs           # Application state, update, view
│   ├── theme.rs         # Pink-brown palette + all style impls
│   ├── editor.rs        # Text editor widget, syntax highlight, tabs
│   ├── pdf_viewer.rs    # PDF rendering panel (pdfium-render)
│   ├── compiler.rs      # LaTeX compiler runner + log parser
│   ├── synctex.rs       # SyncTeX PDF↔source mapping
│   ├── project.rs       # Project model, file management
│   └── file_tree.rs     # Sidebar file tree widget
├── assets/
│   └── fonts/           # JetBrains Mono (auto-downloaded)
├── Cargo.toml
├── build.rs
├── install.sh           # One-shot build + install script
└── README.md
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Ferroleaf (Iced App)                  │
│                                                          │
│  ┌──────────┐  ┌─────────────────┐  ┌────────────────┐  │
│  │ Sidebar  │  │  Editor Panel   │  │   PDF Viewer   │  │
│  │ FileTree │  │  ─────────────  │  │  ─────────────  │  │
│  │ Search   │  │  Tab bar        │  │  pdfium render │  │
│  │          │  │  Line numbers   │  │  Page nav      │  │
│  │          │  │  text_editor    │  │  Zoom          │  │
│  │          │  │  Syntax tokens  │  │  SyncTeX click │  │
│  └──────────┘  └─────────────────┘  └────────────────┘  │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │              Compiler Log Panel                    │  │
│  │   Diagnostics (clickable) │ Raw pdflatex output    │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │   Status bar: cursor pos │ compiler │ build status │  │
│  └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
         │                              │
         ▼                              ▼
   Tokio async               Tokio async
   compiler task             PDF render task
   (pdflatex/xelatex)        (pdfium-render)
         │                              │
         ▼                              ▼
   .synctex.gz               RGBA pixel buffers
   + diagnostics             → iced image handles
```

---

## Color Palette

| Name | Hex | Usage |
|---|---|---|
| BG_DARKEST | `#211717` | Toolbar, status bar |
| BG_DARK | `#2e1f1f` | Sidebar, inactive tabs |
| BG_EDITOR | `#261a1a` | Editor background |
| BG_MID | `#382524` | Panels, cards |
| PINK_BRIGHT | `#f28ca6` | Primary accent, active items |
| PINK_MID | `#cc6b85` | Buttons, highlights |
| BROWN_LIGHT | `#bf8c7a` | Warm highlights |
| TEXT_PRIMARY | `#f2e0db` | Main text |

---

## Troubleshooting

**"No LaTeX installation found"**  
Install TeX Live: `sudo apt-get install texlive-full`

**PDF not rendering**  
Ensure `libpdfium.so` is in the same directory as the `ferroleaf` binary, or set `PDFIUM_DYNAMIC_LIB_PATH`.

**SyncTeX not working**  
Install the synctex tool: `sudo apt-get install synctex`. Compile at least once to generate `.synctex.gz`.

**Build fails on font includes**  
Run `install.sh` which downloads JetBrains Mono automatically, or manually place any `.ttf` file at `assets/fonts/JetBrainsMono-Regular.ttf`.

---

## License

MIT License — use freely, modify freely, no strings attached.

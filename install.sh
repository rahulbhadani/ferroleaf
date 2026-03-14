#!/usr/bin/env bash
set -e
BOLD="\033[1m"; PINK="\033[35m"; GREEN="\033[32m"; YELLOW="\033[33m"; RED="\033[31m"; RESET="\033[0m"
info()    { echo -e "${PINK}${BOLD}[ferroleaf]${RESET} $*"; }
success() { echo -e "${GREEN}${BOLD}[✔]${RESET} $*"; }
warn()    { echo -e "${YELLOW}${BOLD}[⚠]${RESET} $*"; }
error()   { echo -e "${RED}${BOLD}[✖]${RESET} $*"; exit 1; }

#  Rust 
if ! command -v cargo &>/dev/null; then
    info "Installing Rust via rustup…"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi
success "Rust: $(rustc --version)"

#  Package manager 
if   command -v apt-get &>/dev/null; then PKG=apt
elif command -v dnf     &>/dev/null; then PKG=dnf
elif command -v pacman  &>/dev/null; then PKG=pacman
else PKG=unknown; fi

try_install() {
    case $PKG in
        apt)    sudo apt-get install -y "$1" 2>/dev/null ;;
        dnf)    sudo dnf install -y "$1" 2>/dev/null ;;
        pacman) sudo pacman -S --noconfirm "$1" 2>/dev/null ;;
    esac
}

#  Build deps 
info "Installing build dependencies…"
case $PKG in
    apt)    sudo apt-get install -y build-essential pkg-config libssl-dev libgtk-3-dev \
                libx11-dev libxcb1-dev libxkbcommon-dev libvulkan-dev 2>/dev/null || true ;;
    dnf)    sudo dnf install -y gcc openssl-devel gtk3-devel libX11-devel \
                libxkbcommon-devel vulkan-loader-devel 2>/dev/null || true ;;
    pacman) sudo pacman -S --noconfirm base-devel openssl gtk3 libx11 \
                libxcb xkbcommon vulkan-icd-loader 2>/dev/null || true ;;
esac

#  Folder dialog tool 
info "Checking folder dialog tool (zenity/kdialog)…"
if ! command -v zenity &>/dev/null && ! command -v kdialog &>/dev/null; then
    info "Installing zenity…"
    try_install zenity || try_install kdialog || \
        warn "No dialog tool found — install zenity: sudo apt-get install zenity"
else
    success "Dialog tool: $(command -v zenity || command -v kdialog)"
fi

#  PDF renderer (poppler-utils) 
info "Checking PDF renderer (pdftoppm)…"
if ! command -v pdftoppm &>/dev/null; then
    info "Installing poppler-utils…"
    try_install poppler-utils || try_install poppler || \
        warn "pdftoppm not found — PDF preview will not work until you install poppler-utils"
else
    success "PDF renderer: $(command -v pdftoppm)"
fi

#  LaTeX 
info "Checking LaTeX…"
if command -v pdflatex &>/dev/null || command -v xelatex &>/dev/null; then
    success "LaTeX: $(command -v pdflatex || command -v xelatex)"
else
    warn "LaTeX not found. Installing texlive (this may take a while)…"
    case $PKG in
        apt)    sudo apt-get install -y texlive-full 2>/dev/null || \
                sudo apt-get install -y texlive texlive-latex-extra 2>/dev/null || true ;;
        dnf)    sudo dnf install -y texlive-scheme-full 2>/dev/null || true ;;
        pacman) sudo pacman -S --noconfirm texlive-most 2>/dev/null || true ;;
    esac
fi

#  synctex 
info "Checking synctex…"
if ! command -v synctex &>/dev/null; then
    try_install synctex || try_install texlive-binextra || try_install texlive-synctex || \
        warn "synctex not found — PDF→source navigation won't work"
        warn "On Ubuntu: sudo apt-get install texlive-binextra"
else
    success "synctex: $(command -v synctex)"
fi

#  Fonts 
info "Setting up fonts…"
mkdir -p assets/fonts
FONT_OK=false
[ -f assets/fonts/JetBrainsMono-Regular.ttf ] && [ -f assets/fonts/JetBrainsMono-Bold.ttf ] && FONT_OK=true

if $FONT_OK; then
    success "Fonts already present"
else
    # 1. Try system package
    for pkg in fonts-jetbrains-mono jetbrains-mono-fonts ttf-jetbrains-mono; do
        try_install "$pkg" 2>/dev/null && break
    done
    # 2. Search system
    REG=$(find /usr/share/fonts /usr/local/share/fonts ~/.fonts 2>/dev/null \
          -name "JetBrainsMono-Regular.ttf" | head -1)
    BLD=$(find /usr/share/fonts /usr/local/share/fonts ~/.fonts 2>/dev/null \
          -name "JetBrainsMono-Bold.ttf" | head -1)
    if [ -n "$REG" ] && [ -n "$BLD" ]; then
        cp "$REG" assets/fonts/JetBrainsMono-Regular.ttf
        cp "$BLD" assets/fonts/JetBrainsMono-Bold.ttf
        success "Copied JetBrains Mono from system"
        FONT_OK=true
    fi
    # 3. Download
    if ! $FONT_OK; then
        info "Downloading JetBrains Mono…"
        TMP=$(mktemp -d)
        if curl -fsSL "https://github.com/JetBrains/JetBrainsMono/releases/download/v2.304/JetBrainsMono-2.304.zip" \
                -o "$TMP/jb.zip"; then
            unzip -q "$TMP/jb.zip" "fonts/ttf/JetBrainsMono-Regular.ttf" \
                                    "fonts/ttf/JetBrainsMono-Bold.ttf" -d "$TMP/" 2>/dev/null
            [ -f "$TMP/fonts/ttf/JetBrainsMono-Regular.ttf" ] && \
                cp "$TMP/fonts/ttf/JetBrainsMono-Regular.ttf" assets/fonts/ && \
                cp "$TMP/fonts/ttf/JetBrainsMono-Bold.ttf"    assets/fonts/ && \
                FONT_OK=true && success "JetBrains Mono downloaded"
        fi
        rm -rf "$TMP"
    fi
    # 4. Any monospace fallback
    if ! $FONT_OK; then
        FB=$(find /usr/share/fonts -name "*.ttf" 2>/dev/null \
             | grep -iE "mono|courier|consol|hack|fira|liberation" | head -1)
        [ -z "$FB" ] && FB=$(find /usr/share/fonts -name "*.ttf" 2>/dev/null | head -1)
        if [ -n "$FB" ]; then
            cp "$FB" assets/fonts/JetBrainsMono-Regular.ttf
            cp "$FB" assets/fonts/JetBrainsMono-Bold.ttf
            warn "Using fallback font: $FB"
            FONT_OK=true
        else
            error "No TTF font found. Place any .ttf at assets/fonts/JetBrainsMono-Regular.ttf"
        fi
    fi
fi

#  Build 
info "Building Ferroleaf (release)…"
cargo build --release 2>&1 && success "Build complete: target/release/ferroleaf" \
    || error "Build failed"

#  Install 
echo ""
read -p "$(echo -e "${PINK}${BOLD}Install to /usr/local/bin? [y/N]${RESET} ")" -n 1 -r; echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    sudo install -m755 target/release/ferroleaf /usr/local/bin/ferroleaf

    # Install icon
    for size in 16 32 48 64 128 256; do
        ICON_DIR="/usr/share/icons/hicolor/${size}x${size}/apps"
        sudo mkdir -p "$ICON_DIR"
        if command -v rsvg-convert &>/dev/null; then
            rsvg-convert -w $size -h $size assets/icons/ferroleaf.svg \
                | sudo tee "$ICON_DIR/ferroleaf.png" >/dev/null
        elif command -v inkscape &>/dev/null; then
            inkscape -w $size -h $size assets/icons/ferroleaf.svg \
                -o /tmp/ferroleaf_${size}.png 2>/dev/null
            sudo cp /tmp/ferroleaf_${size}.png "$ICON_DIR/ferroleaf.png"
        fi
    done
    # Also install scalable SVG
    sudo mkdir -p /usr/share/icons/hicolor/scalable/apps
    sudo cp assets/icons/ferroleaf.svg /usr/share/icons/hicolor/scalable/apps/ferroleaf.svg
    command -v gtk-update-icon-cache &>/dev/null && \
        sudo gtk-update-icon-cache /usr/share/icons/hicolor/ 2>/dev/null || true

    # Desktop entry with icon
    sudo tee /usr/share/applications/ferroleaf.desktop >/dev/null << DESKTOP
[Desktop Entry]
Name=Ferroleaf
GenericName=LaTeX Editor
Comment=Native LaTeX editor for Linux
Exec=ferroleaf %f
Icon=ferroleaf
Terminal=false
Type=Application
Categories=Office;TextEditor;Education;
MimeType=text/x-tex;application/x-latex;
Keywords=LaTeX;TeX;Editor;PDF;
StartupWMClass=ferroleaf
DESKTOP

    sudo update-desktop-database 2>/dev/null || true
    success "Ferroleaf installed with icon!"
fi

echo ""
success "Done! Launch: ./target/release/ferroleaf"
echo ""
echo -e "${PINK}Tips:${RESET}"
echo "  ★ Click the ★ star next to any .tex file to set it as the main compile target"
echo "  ▶ Ctrl+B compiles the main file (or active .tex file if none set)"
echo "  PDF preview requires: poppler-utils (pdftoppm)"
echo "  Click on PDF text to jump to the source line (requires synctex)"

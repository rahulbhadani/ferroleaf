//! Native folder-picker dialogs for Linux.
//!
//! Tries (in order): zenity, kdialog, qarma, xdg-portal via python3.
//! All are spawned as child processes — no GTK main-loop requirement,
//! works from any thread, works in both GNOME and KDE sessions.

use std::path::PathBuf;
use tokio::process::Command;

/// Pick a folder. Returns None if the user cancelled or no dialog tool found.
pub async fn pick_folder(title: &str) -> Option<PathBuf> {
    // Try each tool in preference order
    if let Some(p) = try_zenity(title).await  { return Some(p); }
    if let Some(p) = try_kdialog(title).await  { return Some(p); }
    if let Some(p) = try_qarma(title).await    { return Some(p); }
    if let Some(p) = try_python_portal(title).await { return Some(p); }
    if let Some(p) = try_yad(title).await      { return Some(p); }
    None
}

async fn try_zenity(title: &str) -> Option<PathBuf> {
    let out = Command::new("zenity")
        .args(["--file-selection", "--directory", "--title", title])
        .output().await.ok()?;
    if out.status.success() {
        let s = String::from_utf8(out.stdout).ok()?;
        let s = s.trim();
        if !s.is_empty() { return Some(PathBuf::from(s)); }
    }
    None
}

async fn try_kdialog(title: &str) -> Option<PathBuf> {
    let out = Command::new("kdialog")
        .args(["--getexistingdirectory", ".", "--title", title])
        .output().await.ok()?;
    if out.status.success() {
        let s = String::from_utf8(out.stdout).ok()?;
        let s = s.trim();
        if !s.is_empty() { return Some(PathBuf::from(s)); }
    }
    None
}

async fn try_qarma(title: &str) -> Option<PathBuf> {
    let out = Command::new("qarma")
        .args(["--file-selection", "--directory", "--title", title])
        .output().await.ok()?;
    if out.status.success() {
        let s = String::from_utf8(out.stdout).ok()?;
        let s = s.trim();
        if !s.is_empty() { return Some(PathBuf::from(s)); }
    }
    None
}

async fn try_yad(title: &str) -> Option<PathBuf> {
    let out = Command::new("yad")
        .args(["--file-selection", "--directory", "--title", title])
        .output().await.ok()?;
    if out.status.success() {
        let s = String::from_utf8(out.stdout).ok()?;
        let s = s.trim();
        if !s.is_empty() { return Some(PathBuf::from(s)); }
    }
    None
}

/// XDG Desktop Portal via python3-gi — works on Wayland/GNOME without zenity.
async fn try_python_portal(title: &str) -> Option<PathBuf> {
    let script = format!(r#"
import gi, sys
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk
dialog = Gtk.FileChooserDialog(
    title="{title}",
    action=Gtk.FileChooserAction.SELECT_FOLDER,
)
dialog.add_buttons(Gtk.STOCK_CANCEL, Gtk.ResponseType.CANCEL,
                   "Select",        Gtk.ResponseType.OK)
response = dialog.run()
if response == Gtk.ResponseType.OK:
    print(dialog.get_filename())
dialog.destroy()
"#, title = title.replace('"', "'"));

    let out = Command::new("python3")
        .args(["-c", &script])
        .output().await.ok()?;
    if out.status.success() {
        let s = String::from_utf8(out.stdout).ok()?;
        let s = s.trim();
        if !s.is_empty() { return Some(PathBuf::from(s)); }
    }
    None
}

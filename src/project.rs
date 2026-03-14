use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

const HIDDEN_EXT: &[&str] = &[
    "aux","log","out","toc","lof","lot","bbl","blg","fls","fdb_latexmk",
    "idx","ilg","ind","ist","glo","gls","glg","acn","acr","alg","xdy",
    "run.xml","bcf","nav","snm","vrb","dvi","ps","tdo",
];

#[derive(Debug, Clone)]
pub struct Project {
    pub root: PathBuf,
    pub name: String,
    /// Explicitly pinned main file (set by user via ★ or saved in .ferroleaf.json).
    /// When None, compile_target() returns the active .tex tab instead.
    pub main_file: Option<PathBuf>,
    pub open_files: Vec<PathBuf>,
    pub active_file: Option<PathBuf>,
    file_contents: HashMap<PathBuf, String>,
    dirty: HashMap<PathBuf, bool>,
}

impl Project {
    pub fn new(root: PathBuf) -> Self {
        let name = root.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string();
        let main_file = find_main(&root);
        Project {
            root, name, main_file,
            open_files: vec![],
            active_file: None,
            file_contents: HashMap::new(),
            dirty: HashMap::new(),
        }
    }

    pub fn visible_files(&self) -> Vec<FileEntry> {
        let mut out = vec![];
        collect(&self.root, &mut out, 0, &self.main_file);
        out
    }

    pub fn open_file(&mut self, path: PathBuf) -> anyhow::Result<()> {
        if !self.open_files.contains(&path) {
            let content = std::fs::read_to_string(&path)?;
            self.file_contents.insert(path.clone(), content);
            self.open_files.push(path.clone());
            self.dirty.insert(path.clone(), false);
        }
        self.active_file = Some(path);
        Ok(())
    }

    pub fn save_file(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        if let Some(c) = self.file_contents.get(path) {
            std::fs::write(path, c)?;
            self.dirty.insert(path.clone(), false);
        }
        Ok(())
    }

    pub fn save_all(&mut self) -> anyhow::Result<()> {
        let dirty: Vec<PathBuf> = self.dirty.iter()
            .filter(|(_, &d)| d)
            .map(|(p, _)| p.clone())
            .collect();
        for p in dirty { self.save_file(&p)?; }
        Ok(())
    }

    pub fn close_file(&mut self, path: &PathBuf) {
        self.open_files.retain(|p| p != path);
        if self.active_file.as_ref() == Some(path) {
            self.active_file = self.open_files.last().cloned();
        }
    }

    pub fn update_content(&mut self, path: &PathBuf, content: String) {
        self.file_contents.insert(path.clone(), content);
        self.dirty.insert(path.clone(), true);
    }

    pub fn get_content(&self, path: &PathBuf) -> Option<&String> {
        self.file_contents.get(path)
    }

    pub fn is_dirty(&self, path: &PathBuf) -> bool {
        self.dirty.get(path).copied().unwrap_or(false)
    }

    /// What gets compiled when the user presses Compile:
    ///
    /// 1. Active tab is a .tex file  →  compile that file (always)
    /// 2. Active tab is not .tex (e.g. .bib) and a ★ main file is pinned  →  use main
    /// 3. No active .tex tab but a ★ main file is pinned  →  use main
    /// 4. None of the above  →  caller shows a warning
    ///
    /// This means the active tab ALWAYS takes priority over the pinned main file,
    /// so switching tabs and pressing Compile does exactly what you expect.
    /// The ★ pin only kicks in when the active tab is not a .tex file.
    pub fn compile_target(&self) -> Option<&PathBuf> {
        // Active .tex tab always wins
        if let Some(ref af) = self.active_file {
            if af.extension().and_then(|e| e.to_str()) == Some("tex") {
                return Some(af);
            }
        }
        // Fallback: pinned main file (active tab is non-tex or nothing open)
        if let Some(ref mf) = self.main_file {
            return Some(mf);
        }
        None
    }

    /// Pin a file as the permanent main file. Persisted in .ferroleaf.json.
    pub fn set_main_file(&mut self, path: PathBuf) {
        self.main_file = Some(path);
    }

    pub fn create_tex_file(&mut self, name: &str) -> anyhow::Result<PathBuf> {
        let fname = if name.ends_with(".tex") {
            name.to_string()
        } else {
            format!("{}.tex", name)
        };
        let path = self.root.join(&fname);
        std::fs::write(&path, format!("% {}\n", fname))?;
        Ok(path)
    }
}

/// Auto-detect the main file when opening a project.
///
/// Rules (in priority order):
/// 1. A file with a conventional main-file name exists at the root.
/// 2. Exactly ONE .tex file at the root contains \documentclass  →  that's the main.
/// 3. Multiple .tex files with \documentclass  →  return None and let the user
///    pick via the ★ star button in the file tree.
fn find_main(root: &Path) -> Option<PathBuf> {
    // Rule 1: conventional names
    for name in &["main.tex","paper.tex","thesis.tex","document.tex","article.tex"] {
        let c = root.join(name);
        if c.exists() { return Some(c); }
    }

    // Rule 2/3: scan for \documentclass
    let candidates: Vec<PathBuf> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().and_then(|x| x.to_str()) == Some("tex")
        })
        .filter(|e| {
            std::fs::read_to_string(e.path())
                .map(|s| s.contains(r"\documentclass"))
                .unwrap_or(false)
        })
        .map(|e| e.path())
        .collect();

    if candidates.len() == 1 {
        Some(candidates[0].clone())
    } else {
        // 0 or 2+ candidates — don't guess, let the user decide
        None
    }
}

fn should_show(path: &Path) -> bool {
    // Hide .synctex.gz (extension would just be "gz" so check full name too)
    if path.to_string_lossy().ends_with(".synctex.gz") { return false; }
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if HIDDEN_EXT.iter().any(|h| *h == ext.to_lowercase()) { return false; }
    }
    true
}

fn collect(dir: &Path, out: &mut Vec<FileEntry>, depth: usize, main: &Option<PathBuf>) {
    let mut items: Vec<_> = WalkDir::new(dir)
        .max_depth(1).min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .collect();

    items.sort_by(|a, b| {
        match (a.file_type().is_dir(), b.file_type().is_dir()) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(b.file_name()),
        }
    });

    for item in items {
        let path = item.path().to_path_buf();
        let name = item.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "target" { continue; }

        if item.file_type().is_dir() {
            out.push(FileEntry { path: path.clone(), name, depth, is_dir: true, is_main: false });
            collect(&path, out, depth + 1, main);
        } else if should_show(&path) {
            let is_main = main.as_ref() == Some(&path);
            out.push(FileEntry { path, name, depth, is_dir: false, is_main });
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
    pub is_main: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectSettings {
    pub main_file: Option<String>,
    #[serde(default = "default_compiler")]
    pub compiler: String,
    #[serde(default)] pub bibtex: bool,
    #[serde(default)] pub shell_escape: bool,
    #[serde(default)] pub extra_args: Vec<String>,
}

fn default_compiler() -> String { "pdflatex".into() }

impl ProjectSettings {
    pub fn load(root: &Path) -> Self {
        let p = root.join(".ferroleaf.json");
        if let Ok(c) = std::fs::read_to_string(&p) {
            serde_json::from_str(&c).unwrap_or_default()
        } else {
            ProjectSettings { compiler: "pdflatex".into(), ..Default::default() }
        }
    }
    pub fn save(&self, root: &Path) -> anyhow::Result<()> {
        let p = root.join(".ferroleaf.json");
        std::fs::write(p, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

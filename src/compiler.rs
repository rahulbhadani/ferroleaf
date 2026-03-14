use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use regex::Regex;

#[derive(Debug, Clone, PartialEq)]
pub enum CompilerKind {
    PdfLatex,
    XeLatex,
    LuaLatex,
}

impl CompilerKind {
    pub fn binary(&self) -> &'static str {
        match self { Self::PdfLatex => "pdflatex", Self::XeLatex => "xelatex", Self::LuaLatex => "lualatex" }
    }
    pub fn display(&self) -> &'static str {
        match self { Self::PdfLatex => "pdfLaTeX", Self::XeLatex => "XeLaTeX", Self::LuaLatex => "LuaLaTeX" }
    }
    pub fn from_str(s: &str) -> Self {
        match s { "xelatex" => Self::XeLatex, "lualatex" => Self::LuaLatex, _ => Self::PdfLatex }
    }
}

#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub compiler: CompilerKind,
    pub shell_escape: bool,
    pub bibtex: bool,
    pub synctex: bool,
    pub extra_args: Vec<String>,
    pub passes: u8,
}

impl Default for CompileOptions {
    fn default() -> Self {
        CompileOptions {
            compiler: CompilerKind::PdfLatex,
            shell_escape: false,
            bibtex: false,
            synctex: true,
            extra_args: vec![],
            passes: 1,
        }
    }
}

#[derive(Debug, Clone)]
pub enum DiagnosticLevel { Error, Warning, Info }

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct CompileResult {
    pub success: bool,
    pub pdf_path: Option<PathBuf>,
    pub diagnostics: Vec<Diagnostic>,
    pub raw_log: String,
    pub duration: Duration,
}

impl CompileResult {
    pub fn errors(&self) -> Vec<&Diagnostic> {
        self.diagnostics.iter().filter(|d| matches!(d.level, DiagnosticLevel::Error)).collect()
    }
    pub fn warnings(&self) -> Vec<&Diagnostic> {
        self.diagnostics.iter().filter(|d| matches!(d.level, DiagnosticLevel::Warning)).collect()
    }
}

#[derive(Debug, Clone)]
pub enum CompileStatus {
    Idle,
    Compiling { pass: u8, total: u8 },
    RunningBibtex,
    Success(CompileResult),
    Failed(CompileResult),
}

impl CompileStatus {
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Compiling { .. } | Self::RunningBibtex)
    }
    pub fn last_result(&self) -> Option<&CompileResult> {
        match self { Self::Success(r) | Self::Failed(r) => Some(r), _ => None }
    }
}

pub struct Compiler;

impl Compiler {
    pub fn available_compilers() -> Vec<CompilerKind> {
        [CompilerKind::PdfLatex, CompilerKind::XeLatex, CompilerKind::LuaLatex]
            .into_iter()
            .filter(|c| which(c.binary()).is_some())
            .collect()
    }

    pub fn is_latex_available() -> bool {
        which("pdflatex").is_some() || which("xelatex").is_some() || which("lualatex").is_some()
    }

    pub async fn compile(
        tex_path: &Path,
        opts: &CompileOptions,
        status_tx: mpsc::Sender<CompileStatus>,
    ) -> CompileResult {
        let start = Instant::now();
        let work_dir = tex_path.parent().unwrap_or(Path::new("."));
        let stem = tex_path.file_stem().unwrap().to_string_lossy().to_string();

        let total = if opts.bibtex { opts.passes + 1 } else { opts.passes };

        let _ = status_tx.send(CompileStatus::Compiling { pass: 1, total }).await;
        let mut result = run_latex(tex_path, work_dir, opts);

        if opts.bibtex {
            let _ = status_tx.send(CompileStatus::RunningBibtex).await;
            run_bibtex(work_dir, &stem);
            let _ = status_tx.send(CompileStatus::Compiling { pass: 2, total }).await;
            run_latex(tex_path, work_dir, opts);
        }

        for i in 1..opts.passes {
            let pass = if opts.bibtex { i + 2 } else { i + 1 };
            let _ = status_tx.send(CompileStatus::Compiling { pass, total }).await;
            result = run_latex(tex_path, work_dir, opts);
        }

        result.duration = start.elapsed();
        let pdf = work_dir.join(format!("{}.pdf", stem));
        if pdf.exists() { result.pdf_path = Some(pdf); result.success = true; }
        result
    }
}

fn run_latex(tex: &Path, cwd: &Path, opts: &CompileOptions) -> CompileResult {
    let mut cmd = Command::new(opts.compiler.binary());
    cmd.current_dir(cwd)
        .arg("-interaction=nonstopmode")
        .arg("-halt-on-error");
    if opts.synctex { cmd.arg("-synctex=1"); }
    if opts.shell_escape { cmd.arg("--shell-escape"); }
    for a in &opts.extra_args { cmd.arg(a); }
    cmd.arg(tex.to_string_lossy().as_ref());

    match cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).output() {
        Ok(out) => {
            let log = String::from_utf8_lossy(&out.stdout).to_string()
                + &String::from_utf8_lossy(&out.stderr);
            let diags = parse_log(&log);
            CompileResult { success: out.status.success(), pdf_path: None, diagnostics: diags, raw_log: log, duration: Duration::ZERO }
        }
        Err(e) => CompileResult {
            success: false, pdf_path: None,
            diagnostics: vec![Diagnostic { level: DiagnosticLevel::Error, file: None, line: None,
                message: format!("Failed to run {}: {}", opts.compiler.binary(), e) }],
            raw_log: format!("{}", e), duration: Duration::ZERO,
        },
    }
}

fn run_bibtex(cwd: &Path, stem: &str) {
    let _ = Command::new("bibtex").current_dir(cwd).arg(stem).output();
}

fn which(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let f = dir.join(name); if f.is_file() { Some(f) } else { None }
        })
    })
}

fn parse_log(log: &str) -> Vec<Diagnostic> {
    let mut diags = vec![];
    let err_re  = Regex::new(r"^!\s+(.+)").unwrap();
    let line_re = Regex::new(r"l\.(\d+)").unwrap();
    let warn_re = Regex::new(r"(?i)(LaTeX Warning|Package \w+ Warning):\s*(.+)").unwrap();
    let over_re = Regex::new(r"(Overfull|Underfull) \\[hv]box").unwrap();

    let lines: Vec<&str> = log.lines().collect();
    for (i, &line) in lines.iter().enumerate() {
        if let Some(cap) = err_re.captures(line) {
            let line_num = lines[i..].iter().take(5)
                .find_map(|l| line_re.captures(l))
                .and_then(|c| c[1].parse::<u32>().ok());
            diags.push(Diagnostic { level: DiagnosticLevel::Error, file: None, line: line_num, message: cap[1].trim().into() });
        }
        if let Some(cap) = warn_re.captures(line) {
            diags.push(Diagnostic { level: DiagnosticLevel::Warning, file: None, line: None, message: cap[2].trim().into() });
        }
        if over_re.is_match(line) {
            diags.push(Diagnostic { level: DiagnosticLevel::Info, file: None, line: None, message: line.trim().into() });
        }
    }
    diags
}

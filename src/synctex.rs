use std::path::{Path, PathBuf};
use std::process::Command;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: PathBuf,
    pub line: u32,
    pub column: u32,
}

/// Resolve a PDF click position to a LaTeX source location using SyncTeX.
pub fn pdf_to_source(pdf_path: &Path, page: u32, x: f32, y: f32) -> Option<SourceLocation> {
    // Try synctex binary first
    synctex_edit_bin(pdf_path, page, x, y)
        .or_else(|| synctex_parse(pdf_path, page, x, y))
}

fn synctex_edit_bin(pdf: &Path, page: u32, x: f32, y: f32) -> Option<SourceLocation> {
    let out = Command::new("synctex")
        .arg("edit")
        .arg("-o")
        .arg(format!("{}:{}:{}:{}", page, x, y, pdf.display()))
        .output()
        .ok()?;
    parse_edit_output(&String::from_utf8_lossy(&out.stdout))
}

fn parse_edit_output(s: &str) -> Option<SourceLocation> {
    let out_re  = Regex::new(r"^Output:(.+)$").ok()?;
    let line_re = Regex::new(r"^Line:(\d+)$").ok()?;
    let col_re  = Regex::new(r"^Column:(\d+)$").ok()?;
    let mut file = None; let mut line = None; let mut col = None;
    for l in s.lines() {
        if let Some(c) = out_re.captures(l)  { file = Some(PathBuf::from(c[1].trim())); }
        if let Some(c) = line_re.captures(l) { line = c[1].parse().ok(); }
        if let Some(c) = col_re.captures(l)  { col  = c[1].parse().ok(); }
    }
    Some(SourceLocation { file: file?, line: line.unwrap_or(1), column: col.unwrap_or(0) })
}

fn synctex_parse(pdf: &Path, target_page: u32, tx: f32, ty: f32) -> Option<SourceLocation> {
    use std::io::Read;
    let gz = pdf.with_extension("synctex.gz");
    let plain = pdf.with_extension("synctex");
    let content = if gz.exists() {
        let f = std::fs::File::open(&gz).ok()?;
        let mut dec = flate2::read::GzDecoder::new(f);
        let mut s = String::new();
        dec.read_to_string(&mut s).ok()?; s
    } else if plain.exists() {
        std::fs::read_to_string(&plain).ok()?
    } else {
        return None;
    };
    nearest_record(&content, target_page, tx, ty)
}

fn nearest_record(content: &str, target_page: u32, tx: f32, ty: f32) -> Option<SourceLocation> {
    let mut files: std::collections::HashMap<u32, PathBuf> = std::collections::HashMap::new();
    let inp_re  = Regex::new(r"^Input:(\d+):(.+)$").ok()?;
    let rec_re  = Regex::new(r"^[hHvV]:(\d+),(\d+):(\d+),(\d+)").ok()?;
    let page_re = Regex::new(r"^\{(\d+)").ok()?;

    let mut cur_page = 0u32;
    let mut best: Option<(f32, u32, u32)> = None; // (dist, input_id, line)

    for line in content.lines() {
        if let Some(c) = inp_re.captures(line) {
            let id: u32 = c[1].parse().unwrap_or(0);
            files.insert(id, PathBuf::from(&c[2]));
            continue;
        }
        if let Some(c) = page_re.captures(line) {
            cur_page = c[1].parse().unwrap_or(0);
            continue;
        }
        if cur_page != target_page { continue; }
        if let Some(c) = rec_re.captures(line) {
            let inp: u32 = c[1].parse().unwrap_or(0);
            let src: u32 = c[2].parse().unwrap_or(0);
            let rx: f32  = c[3].parse::<f32>().unwrap_or(0.0) / 65536.0;
            let ry: f32  = c[4].parse::<f32>().unwrap_or(0.0) / 65536.0;
            let d = ((rx - tx).powi(2) + (ry - ty).powi(2)).sqrt();
            if best.as_ref().map(|(bd, ..)| d < *bd).unwrap_or(true) {
                best = Some((d, inp, src));
            }
        }
    }
    let (_, inp, line) = best?;
    Some(SourceLocation { file: files.get(&inp)?.clone(), line, column: 0 })
}

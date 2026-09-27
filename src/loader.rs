use regex::Regex;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn load_prospectus_text() -> (String, String) {
    let root = project_root();
    let candidates = [
        root.join("2027-Prospectus-080526.docx"),
        root.join("data").join("2027-Prospectus-080526.docx"),
        root.join("assets").join("prospectus_2027.txt"),
    ];

    for path in &candidates {
        if path.extension().and_then(|e| e.to_str()) == Some("docx") {
            if let Ok(text) = load_docx(path) {
                return (text, path.display().to_string());
            }
        } else if let Ok(text) = fs::read_to_string(path) {
            return (text, path.display().to_string());
        }
    }

    // Also scan data/ and the project root for any prospectus docx
    for dir in [root.clone(), root.join("data")] {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                if name.ends_with(".docx") && name.contains("prospectus") {
                    if let Ok(text) = load_docx(&p) {
                        return (text, p.display().to_string());
                    }
                }
            }
        }
    }

    (String::new(), String::new())
}

pub fn load_docx(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let file = fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut doc_xml = archive.by_name("word/document.xml")?;
    let mut content = String::new();
    doc_xml.read_to_string(&mut content)?;
    content = content
        .replace("&amp;", "&")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("</w:p>", "\n")
        .replace("</w:tc>", "\n")
        .replace("</w:tr>", "\n");
    let re = Regex::new(r"<[^>]+>")?;
    Ok(re.replace_all(&content, " ").to_string())
}

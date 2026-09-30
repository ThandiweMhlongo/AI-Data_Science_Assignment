use regex::Regex;
use std::collections::{HashMap, HashSet};

use crate::aps::{
    calculate_aps_only, format_aps_howto, format_programme, load_programmes, looks_like_aps_howto, looks_like_results,
    match_programmes, parse_marks, predict_courses, ProgrammeAps,
};

use crate::course_info::{
    course_information,
    looks_like_course_info,
};

use crate::loader::load_prospectus_text;
#[derive(Clone, Debug)]
pub struct Chunk {
    pub id: usize,
    pub title: String,
    pub text: String,
}

pub struct RagIndex {
    pub source: String,
    pub chunks: Vec<Chunk>,
    pub programmes: Vec<ProgrammeAps>,
    avg_len: f32,
    df: HashMap<String, usize>,
}

impl RagIndex {
    pub fn load() -> Self {
        let (text, source) = load_prospectus_text();
        let chunks = chunk_document(&text);
        let mut df: HashMap<String, usize> = HashMap::new();
        let mut total_len = 0usize;
        for chunk in &chunks {
            total_len += tokenize(&chunk.text).len();
            let mut seen = HashSet::new();
            for tok in tokenize(&chunk.text) {
                if seen.insert(tok.clone()) {
                    *df.entry(tok).or_insert(0) += 1;
                }
            }
        }
        let avg_len = if chunks.is_empty() {
            1.0
        } else {
            total_len as f32 / chunks.len() as f32
        };
        Self {
            source,
            chunks,
            programmes: load_programmes(),
            avg_len,
            df,
        }
    }

    pub fn answer(&self, question: &str) -> String {
        let q = question.trim();
        if q.is_empty() {
            return "Ask a question from the official CPUT prospectus: APS, entry requirements, fees, deadlines, modules, or a specific qualification.".to_string();
        }
        if self.chunks.is_empty() {
            return "The 2027 prospectus could not be loaded. Keep 2027-Prospectus-080526.docx in the CHATBOX_CPUT_PROSPECTUS folder.".to_string();
        }

       let marks = parse_marks(q);

// APS CALCULATOR PAGE
if q.to_lowercase().starts_with("calculate my aps:") {
    return calculate_aps_only(&marks);
}

// COURSE CHECKER LANDING PAGE
if marks.len() >= 3 || looks_like_results(q) {
    return predict_courses(&marks, &self.programmes);
}
// COURSE INFORMATION PAGE
if looks_like_course_info(q) {

    let course_query = q
        .trim()
        .strip_prefix("COURSE_INFO:")
        .unwrap_or(q)
        .trim();

    // First try the structured APS programme records.
    if crate::course_info::find_course(q, &self.programmes).is_some() {
    return course_information(q, &self.programmes);
}

    // If not found there, search the actual prospectus.
    let hits = self.retrieve(course_query, 8);

    if hits.is_empty() {
        return format!(
            "I could not find information for \"{}\" \
in the 2027 CPUT prospectus.\n\n\
Please check the qualification name and try again.",
            course_query
        );
    }

    return synthesize_course_information(
        course_query,
        &hits,
        &self.source,
    );
}
        let matched = match_programmes(q, &self.programmes);
        if !matched.is_empty() {
            return matched
                .iter()
                .map(|p| format_programme(p))
                .collect::<Vec<_>>()
                .join("\n\n--------------------\n\n");
        }

        let hits = self.retrieve(&expand_query(q), 5);
        if hits.is_empty() {
            return "I could not find that in the indexed 2027 CPUT prospectus. Try a qualification name, APS, application steps, fees, or a faculty.".to_string();
        }

        synthesize(q, &hits, &self.source)
    }

    fn retrieve(&self, question: &str, k: usize) -> Vec<(f32, &Chunk)> {
        let q_tokens = tokenize(question);
        if q_tokens.is_empty() {
            return Vec::new();
        }
        let n = self.chunks.len() as f32;
        let mut scored: Vec<(f32, &Chunk)> = Vec::new();
        for chunk in &self.chunks {
            let tokens = tokenize(&chunk.text);
            if tokens.is_empty() {
                continue;
            }
            let mut tf: HashMap<&str, f32> = HashMap::new();
            for t in &tokens {
                *tf.entry(t.as_str()).or_insert(0.0) += 1.0;
            }
            let dl = tokens.len() as f32;
            let mut score = 0.0f32;
            let k1 = 1.5f32;
            let b = 0.75f32;
            for qt in &q_tokens {
                let f = *tf.get(qt.as_str()).unwrap_or(&0.0);
                if f == 0.0 {
                    continue;
                }
                let df = *self.df.get(qt).unwrap_or(&0) as f32;
                let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln().max(0.0);
                let denom = f + k1 * (1.0 - b + b * (dl / self.avg_len.max(1.0)));
                score += idf * (f * (k1 + 1.0)) / denom;
                if chunk.title.to_lowercase().contains(qt) {
                    score += 1.2;
                }
            }
            if score > 0.4 {
                scored.push((score, chunk));
            }
        }
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        scored
    }
}

pub fn chunk_document(text: &str) -> Vec<Chunk> {
    let header_re = Regex::new(
        r"(?i)^(DIPLOMA|BACHELOR|HIGHER CERTIFICATE|ADVANCED DIPLOMA|BED |BHSC|BSC |HC IN|FACULTY OF|CALCULATING THE APS|METHOD [123]|ONLINE APPLICATIONS|WHAT WILL IT COST|DO YOU NEED FINANCIAL|STUDY VISAS|EARLY CLOSING|MINIMUM APS|DEPARTMENT QUALIFICATION)",
    )
    .unwrap();

    let mut chunks = Vec::new();
    let mut title = "CPUT Undergraduate Prospectus 2027".to_string();
    let mut buf: Vec<String> = Vec::new();

    for raw in text.lines() {
        let line = collapse(raw);
        if line.is_empty() || is_junk(&line) {
            continue;
        }
        if header_re.is_match(&line) && line.len() < 160 {
            flush(&mut chunks, &title, &mut buf);
            title = line.clone();
        }
        buf.push(line);
        if buf.join(" ").len() > 900 {
            flush(&mut chunks, &title, &mut buf);
        }
    }
    flush(&mut chunks, &title, &mut buf);
    dedupe(chunks)
}

fn flush(chunks: &mut Vec<Chunk>, title: &str, buf: &mut Vec<String>) {
    let text = collapse(&buf.join(" "));
    buf.clear();
    if text.len() < 80 {
        return;
    }
    chunks.push(Chunk {
        id: chunks.len() + 1,
        title: title.to_string(),
        text,
    });
}

fn dedupe(chunks: Vec<Chunk>) -> Vec<Chunk> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for mut c in chunks {
        let key: String = c.text.chars().take(140).collect::<String>().to_lowercase();
        if seen.insert(key) {
            c.id = out.len() + 1;
            out.push(c);
        }
    }
    out
}

fn synthesize(question: &str, hits: &[(f32, &Chunk)], source: &str) -> String {
    let q_tokens = tokenize(question);
    let mut lines: Vec<String> = Vec::new();
    for (_, chunk) in hits {
        let cleaned = strip_noise(&format!("{} {}", chunk.title, chunk.text));
        for sent in split_sentences(&cleaned) {
            let s = collapse(&sent);
            if s.len() < 40 || s.len() > 220 {
                continue;
            }
            if looks_like_table_junk(&s) {
                continue;
            }
            let st = tokenize(&s);
            let overlap = q_tokens.iter().filter(|t| st.iter().any(|w| w == *t)).count();
            if overlap > 0 {
                lines.push(s);
            }
        }
    }
    lines.dedup();
    if lines.is_empty() {
        return "I could not pull a clean subject/APS list for that. Ask using the qualification name, for example: Diploma in Chemical Engineering.".to_string();
    }
    let mut out = String::from("From the 2027 prospectus:\n\n");
    for s in lines.iter().take(5) {
        out.push_str("• ");
        out.push_str(s);
        out.push('\n');
    }
    if !source.is_empty() {
        out.push_str(&format!("\nSource file: {source}"));
    }
    out
}
fn synthesize_course_information(
    course: &str,
    hits: &[(f32, &Chunk)],
    source: &str,
) -> String {

    let course_tokens = tokenize(course);

    let mut relevant: Vec<String> = Vec::new();

    for (_, chunk) in hits {

        let combined =
            format!("{} {}", chunk.title, chunk.text);

        let combined_tokens =
            tokenize(&combined);

        let matches =
            course_tokens
                .iter()
                .filter(|token| {
                    combined_tokens.contains(token)
                })
                .count();

        if matches > 0 {
            relevant.push(
                collapse(&combined)
            );
        }
    }

    relevant.dedup();

    if relevant.is_empty() {
        return format!(
            "I could not find course information for \
\"{}\" in the 2027 CPUT prospectus.",
            course
        );
    }

    let mut response =
        String::new();

    response.push_str(
        "🎓 COURSE INFORMATION\n\n"
    );

    response.push_str(
        &format!("Qualification: {}\n\n", course)
    );

    for text in relevant.iter().take(3) {

        response.push_str(text);

        response.push_str("\n\n");
    }

    if !source.is_empty() {
        response.push_str(
            "Source: CPUT 2027 Prospectus"
        );
    }

    response
}

fn strip_noise(s: &str) -> String {
    Regex::new(r"-?\d{5,}")
        .unwrap()
        .replace_all(s, " ")
        .to_string()
}

fn looks_like_table_junk(s: &str) -> bool {
    let up = s.to_uppercase();
    up.contains("MINIMUM ADMISSION REQUIREMENT")
        || up.contains("TABLE B")
        || up.contains("TABLE A")
        || up.contains("HEQSF")
        || up.contains("SCANNING THIS CODE")
        || (up.matches("MATHS").count() >= 3 && up.contains("APS METHOD"))
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        cur.push(ch);
        if matches!(ch, '.' | '?' | '!') && cur.len() > 50 {
            out.push(cur.trim().to_string());
            cur.clear();
        }
    }
    if collapse(&cur).len() > 40 {
        out.push(cur.trim().to_string());
    }
    if out.is_empty() {
        out.push(text.to_string());
    }
    out
}

fn expand_query(q: &str) -> String {
    let n = q.to_lowercase();
    let mut extra = String::new();
    if n.contains("aps") || n.contains("admission point") {
        extra.push_str(" APS admission point score method calculating excluding life orientation");
    }
    if n.contains("deadline") || n.contains("closing") || n.contains("due date") {
        extra.push_str(" early closing dates application apply online keep checking website");
    }
    if n.contains("fee") || n.contains("tuition") || n.contains("cost") {
        extra.push_str(" fees discount tuition payment instalments international levy");
    }
    if n.contains("apply") || n.contains("application") {
        extra.push_str(" online applications seven steps track apply documents");
    }
    if n.contains("module") || n.contains("subject") || n.contains("requirement") {
        extra.push_str(" admission requirements english mathematics physical science");
    }
    format!("{q} {extra}")
}

fn tokenize(s: &str) -> Vec<String> {
    let stop: HashSet<&str> = [
        "the", "a", "an", "and", "or", "of", "to", "in", "on", "for", "is", "are", "was", "be",
        "at", "by", "with", "from", "as", "it", "this", "that", "i", "you", "we", "do", "does",
        "can", "what", "how", "when", "where", "who", "which", "please", "tell", "me", "about",
        "cput", "university", "my", "need", "get", "have",
    ]
    .into_iter()
    .collect();
    normalize(s)
        .split_whitespace()
        .filter(|w| w.len() > 1 && !stop.contains(w))
        .map(|w| w.trim_end_matches('s').to_string())
        .collect()
}

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c.is_whitespace() { c } else { ' ' })
        .collect::<String>()
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_junk(line: &str) -> bool {
    let letters = line.chars().filter(|c| c.is_ascii_alphabetic()).count();
    let digits = line.chars().filter(|c| c.is_ascii_digit()).count();
    if letters < 6 {
        return true;
    }
    digits > 10 && letters < 14
}

fn truncate(s: &str, n: usize) -> String {
    let mut t: String = s.chars().take(n).collect();
    if s.chars().count() > n {
        t.push('…');
    }
    t
}

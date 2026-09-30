use regex::Regex;
use std::cmp::Ordering;

#[derive(Clone, Debug)]
pub struct SubjectReq {
    pub subject: String,
    pub rating: String,
    pub percent: String,
}

#[derive(Clone, Debug)]
pub struct ProgrammeAps {
    pub name: String,
    pub aps: String,
    pub method: String,
    pub bands: String,
    pub subjects: Vec<SubjectReq>,
}

pub fn load_programmes() -> Vec<ProgrammeAps> {
    parse_records(include_str!("../assets/aps_records.txt"))
}

pub fn parse_records(raw: &str) -> Vec<ProgrammeAps> {
    let mut out = Vec::new();
    let mut cur: Option<ProgrammeAps> = None;
    for line in raw.lines() {
        if let Some(name) = line.strip_prefix("NAME|") {
            if let Some(p) = cur.take() {
                out.push(p);
            }
            cur = Some(ProgrammeAps {
                name: name.to_string(),
                aps: String::new(),
                method: String::new(),
                bands: String::new(),
                subjects: Vec::new(),
            });
        } else if let Some(p) = cur.as_mut() {
            if let Some(v) = line.strip_prefix("APS|") {
                p.aps = v.to_string();
            } else if let Some(v) = line.strip_prefix("METHOD|") {
                p.method = v.to_string();
            } else if let Some(v) = line.strip_prefix("BANDS|") {
                p.bands = v.to_string();
            } else if let Some(rest) = line.strip_prefix("SUB|") {
                let parts: Vec<&str> = rest.splitn(3, '|').collect();
                if parts.len() == 3 {
                    p.subjects.push(SubjectReq {
                        subject: parts[0].to_string(),
                        rating: parts[1].to_string(),
                        percent: parts[2].to_string(),
                    });
                }
            } else if line == "END" {
                if let Some(p) = cur.take() {
                    out.push(p);
                }
            }
        }
    }
    if let Some(p) = cur.take() {
        out.push(p);
    }
    out
}

pub fn format_aps_howto() -> String {
    "How CPUT APS is calculated (Prospectus 2027)\n\
     \n\
     Life Orientation is NOT used.\n\
     You must also meet the subject ratings below for that qualification.\n\
     \n\
     Method 1 — Best of six subjects\n\
     • Take the % marks of your 6 highest subjects (include required subjects such as English, Maths, Physical Science)\n\
     • Add them\n\
     • Divide by 10\n\
     • That number is your APS total\n\
     \n\
     Example (Method 1):\n\
     • English — 57%\n\
     • Mathematics — 55%\n\
     • Subject 3 — 50%\n\
     • Subject 4 — 54%\n\
     • Subject 5 — 43%\n\
     • Subject 6 — 45%\n\
     • Total 304 ÷ 10 = APS 30.4\n\
     \n\
     Method 2 — Double Maths and Science (used by many Engineering programmes)\n\
     • English % as is\n\
     • Mathematics % × 2\n\
     • Physical Science % × 2\n\
     • Next best subject % as is\n\
     • Add them, then divide by 10\n\
     \n\
     Example (Method 2):\n\
     • English — 57%\n\
     • Mathematics — 55% × 2 = 110\n\
     • Physical Science — 50% × 2 = 100\n\
     • Next subject — 54%\n\
     • Total 321 ÷ 10 = APS 32.1\n\
     \n\
     NSC rating to percentage:\n\
     • 7 = 80–100%\n\
     • 6 = 70–79%\n\
     • 5 = 60–69%\n\
     • 4 = 50–59%\n\
     • 3 = 40–49%\n\
     • 2 = 30–39%\n\
     • 1 = 0–29%"
        .to_string()
}

pub fn format_programme(p: &ProgrammeAps) -> String {
    let mut out = format!("{}\n\n", p.name);
    out.push_str(&format!("Total APS needed: {}\n", p.aps));
    if !p.method.trim().is_empty() {
        out.push_str(&format!("APS method: {}\n", p.method));
    }
    let extra = tidy_bands(&p.bands);
    if !extra.is_empty() {
        out.push_str(&format!("{extra}\n"));
    }
    out.push_str("\nSubjects (rating = percentage band):\n");
    for s in &p.subjects {
        if s.rating == "N/A" {
            out.push_str(&format!("• {}: not accepted / not required\n", s.subject));
        } else {
            out.push_str(&format!("• {}: {}  ({})\n", s.subject, s.rating, s.percent));
        }
    }
    out.push_str("\nAsk another qualification name for its APS list.");
    out
}

fn tidy_bands(raw: &str) -> String {
    let mut bits = Vec::new();
    if raw.contains("27") && raw.contains("29") {
        bits.push("ECP / possible range: 27–29".to_string());
    }
    if raw.contains("23") && raw.contains("26") {
        bits.push("Waitlist range: 23–26".to_string());
    }
    bits.join("\n")
}

pub fn match_programmes<'a>(question: &str, programmes: &'a [ProgrammeAps]) -> Vec<&'a ProgrammeAps> {
    let q = norm(question);
    if q.contains("how") && (q.contains("aps") || q.contains("calculat")) {
        return Vec::new();
    }
    let mut scored: Vec<(i32, &ProgrammeAps)> = Vec::new();
    for p in programmes {
        let n = norm(&p.name);
        let mut score = 0i32;
        if q.contains(&n) || n.contains(&q) {
            score += 30;
        }
        for word in n.split_whitespace() {
            if word.len() > 3 && q.contains(word) {
                score += 4;
            }
        }
        if (q.contains("chemical") && n.contains("chemical"))
            || (q.contains("civil") && n.contains("civil"))
            || (q.contains("nursing") && n.contains("nursing"))
            || (q.contains("mechanical") && n.contains("mechanical"))
            || (q.contains("electrical") && n.contains("electrical"))
            || (q.contains("construction") && n.contains("construction"))
            || (q.contains("agriculture") && n.contains("agriculture"))
        {
            score += 12;
        }
        if score >= 12 {
            scored.push((score, p));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));
    scored.dedup_by(|a, b| a.1.name == b.1.name);
    scored.into_iter().take(3).map(|(_, p)| p).collect()
}

fn norm(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c.is_whitespace() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn looks_like_results(q: &str) -> bool {
    let n = norm(q);
    let has_pct = q.contains('%') || Regex::new(r"\b\d{2}\b").ok().is_some_and(|r| r.is_match(q));
    has_pct
        && (n.contains("english")
            || n.contains("math")
            || n.contains("science")
            || n.contains("qualify")
            || n.contains("what course")
            || n.contains("can i")
            || n.contains("my marks")
            || n.contains("my results")
            || n.contains("i got")
            || n.contains("i have"))
}

pub fn parse_marks(question: &str) -> Vec<(String, f32)> {
    let q = question.to_lowercase().replace('%', " ");
    let aliases = [
        ("life orientation", "skip"),
        ("mathematical literacy", "mathematical literacy"),
        ("maths literacy", "mathematical literacy"),
        ("math literacy", "mathematical literacy"),
        ("technical mathematics", "technical maths"),
        ("technical maths", "technical maths"),
        ("tech maths", "technical maths"),
        ("physical sciences", "physical science"),
        ("physical science", "physical science"),
        ("technical science", "technical science"),
        ("tech science", "technical science"),
        ("life sciences", "life sciences"),
        ("life science", "life sciences"),
        ("business studies", "business studies"),
        ("english", "english"),
        ("mathematics", "mathematics"),
        ("maths", "mathematics"),
        ("accounting", "accounting"),
        ("geography", "geography"),
        ("economics", "economics"),
        ("history", "history"),
        ("tourism", "tourism"),
        ("cat", "cat"),
        ("it", "it"),
    ];
    let mut marks = Vec::new();
    let re = Regex::new(r"(?i)([a-z][a-z &/]+?)\s*[:=]?\s*(\d{1,3})").unwrap();
    for cap in re.captures_iter(&q) {
        let raw = collapse(cap.get(1).map(|m| m.as_str()).unwrap_or(""));
        let pct: f32 = cap.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
        if !(1.0..=100.0).contains(&pct) {
            continue;
        }
        let mut key = String::new();
        for (a, canon) in aliases {
            if raw.contains(a) {
                key = canon.to_string();
                break;
            }
        }
        if key.is_empty() || key == "skip" {
            if raw.len() > 3 && !raw.contains("got") && !raw.contains("have") && !raw.contains("course") {
                key = raw;
            } else {
                continue;
            }
        }
        if !marks.iter().any(|(k, _)| k == &key) {
            marks.push((key, pct));
        }
    }
    marks
}

pub fn rating_from_percent(p: f32) -> i32 {
    if p >= 80.0 {
        7
    } else if p >= 70.0 {
        6
    } else if p >= 60.0 {
        5
    } else if p >= 50.0 {
        4
    } else if p >= 40.0 {
        3
    } else if p >= 30.0 {
        2
    } else {
        1
    }
}

fn mark_of(marks: &[(String, f32)], names: &[&str]) -> Option<f32> {
    marks.iter().find(|(k, _)| names.iter().any(|n| k == n)).map(|(_, p)| *p)
}

pub fn aps_method1(marks: &[(String, f32)]) -> f32 {
    let mut vals: Vec<f32> = marks
        .iter()
        .filter(|(k, _)| k != "life orientation")
        .map(|(_, p)| *p)
        .collect();
    vals.sort_by(|a, b| b.partial_cmp(a).unwrap());
    vals.truncate(6);
    if vals.is_empty() {
        return 0.0;
    }
    vals.iter().sum::<f32>() / 10.0
}

pub fn aps_method2(marks: &[(String, f32)]) -> f32 {
    let eng = mark_of(marks, &["english"]).unwrap_or(0.0);
    let maths = mark_of(marks, &["mathematics"]).or_else(|| mark_of(marks, &["technical maths"])).unwrap_or(0.0);
    let sci = mark_of(marks, &["physical science"]).or_else(|| mark_of(marks, &["technical science"])).unwrap_or(0.0);
    let mut rest: Vec<f32> = marks
        .iter()
        .filter(|(k, _)| {
            k != "english" && k != "mathematics" && k != "technical maths" && k != "physical science" && k != "technical science" && k != "life orientation"
        })
        .map(|(_, p)| *p)
        .collect();
    rest.sort_by(|a, b| b.partial_cmp(a).unwrap());
    let next = rest.first().copied().unwrap_or(0.0);
    (eng + maths * 2.0 + sci * 2.0 + next) / 10.0
}

fn required_rating(s: &SubjectReq) -> Option<i32> {
    if s.rating == "N/A" {
        return None;
    }
    let n: i32 = s.rating.parse().ok()?;
    if n <= 1 {
        return None; // parser junk
    }
    Some(n)
}

fn user_rating(marks: &[(String, f32)], names: &[&str]) -> i32 {
    mark_of(marks, names).map(rating_from_percent).unwrap_or(0)
}

pub fn qualifies(p: &ProgrammeAps, marks: &[(String, f32)]) -> Option<(f32, String)> {
    let need: f32 = p
        .aps
        .split(|c: char| !c.is_ascii_digit())
        .find(|s| !s.is_empty())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    if need < 18.0 {
        return None;
    }
    let method2 = p.method.to_lowercase().contains('2');
    let aps = if method2 { aps_method2(marks) } else { aps_method1(marks) };
    if aps + 0.05 < need {
        return None;
    }

    let ml_forbidden = p.subjects.iter().any(|s| {
        s.subject.eq_ignore_ascii_case("Mathematical Literacy") && s.rating == "N/A"
    });
    let has_maths = mark_of(marks, &["mathematics"]).is_some() || mark_of(marks, &["technical maths"]).is_some();
    if ml_forbidden && !has_maths {
        return None;
    }

    for s in &p.subjects {
        let Some(need_r) = required_rating(s) else { continue };
        let key = s.subject.to_lowercase();
        let got = if key.contains("english") {
            user_rating(marks, &["english"])
        } else if key == "mathematics" {
            let m = user_rating(marks, &["mathematics"]);
            let t = {
                let tr = p.subjects.iter().find(|x| x.subject.to_lowercase().contains("technical maths"));
                let need_t = tr.and_then(required_rating).unwrap_or(need_r);
                let got_t = user_rating(marks, &["technical maths"]);
                if got_t >= need_t { got_t } else { 0 }
            };
            m.max(t)
        } else if key.contains("technical maths") {
            continue; // already considered with Mathematics
        } else if key.contains("mathematical literacy") {
            if ml_forbidden {
                continue;
            }
            let maths_need = p
                .subjects
                .iter()
                .find(|x| x.subject.eq_ignore_ascii_case("Mathematics"))
                .and_then(required_rating);
            let tech_need = p
                .subjects
                .iter()
                .find(|x| x.subject.to_lowercase().contains("technical maths"))
                .and_then(required_rating);
            let m = user_rating(marks, &["mathematics"]);
            let t = user_rating(marks, &["technical maths"]);
            if maths_need.is_some_and(|n| m >= n) || tech_need.is_some_and(|n| t >= n) {
                need_r
            } else {
                user_rating(marks, &["mathematical literacy"])
            }
        } else if key.contains("physical science") {
            let ps = user_rating(marks, &["physical science"]);
            let ts = user_rating(marks, &["technical science"]);
            let ts_need = p
                .subjects
                .iter()
                .find(|x| x.subject.to_lowercase().contains("technical science"))
                .and_then(required_rating);
            if ps >= need_r {
                ps
            } else if ts_need.is_some() && ts >= ts_need.unwrap() {
                ts
            } else {
                ps
            }
        } else if key.contains("technical science") {
            continue;
        } else if key.contains("life science") {
            user_rating(marks, &["life sciences"])
        } else {
            user_rating(marks, &[&key])
        };
        if got < need_r {
            return None;
        }
    }

    let has_real = p.subjects.iter().any(|s| required_rating(s).is_some());
    if !has_real && !p.name.to_lowercase().contains("education") {
        return None;
    }

    let label = if method2 { "Method 2" } else { "Method 1" };
    Some((aps, label.to_string()))
}
pub fn calculate_aps_only(marks: &[(String, f32)]) -> String {
    if marks.len() < 3 {
        return "Please enter at least 3 subjects and their percentages.\n\n\
Example:\n\
English 65%, Mathematics 58%, Physical Science 61%, \
Life Sciences 55%, Geography 60%, Business Studies 67%"
            .to_string();
    }

    let method1 = aps_method1(marks);
    let method2 = aps_method2(marks);

    let mut response = String::new();

    response.push_str("📊 YOUR APS SCORE\n\n");

    response.push_str(&format!(
        "APS Method 1: {:.1}\n",
        method1
    ));

    response.push_str(&format!(
        "APS Method 2: {:.1}\n",
        method2
    ));

    response.push_str("\nSubjects entered:\n");

    for (subject, mark) in marks {
        response.push_str(&format!(
            "• {}: {:.0}%\n",
            subject,
            mark
        ));
    }

    response.push_str(
        "\nThis APS calculation is based on the CPUT APS methods."
    );

    response
}

pub fn predict_courses(marks: &[(String, f32)], programmes: &[ProgrammeAps]) -> String {
    if marks.len() < 3 {
        return "Type at least 3 subjects with percentages in the chat box.".to_string();
    }
    let a1 = aps_method1(marks);
    let a2 = aps_method2(marks);
    let mut out = String::from("Your results\n\n");
    for (k, p) in marks {
        out.push_str(&format!("• {} — {:.0}%  (rating {})\n", title_case(k), p, rating_from_percent(*p)));
    }
    out.push_str(&format!("\nYour APS Method 1 (best 6 subjects ÷ 10): {:.1}\n", a1));
    out.push_str(&format!("Your APS Method 2 (double Maths & Science): {:.1}\n", a2));
    out.push_str("Life Orientation is not used.\n\n");

    let mut hits: Vec<(f32, &ProgrammeAps, String)> = Vec::new();
    for p in programmes {
        if let Some((aps, method)) = qualifies(p, marks) {
            hits.push((aps, p, method));
        }
    }
    hits.sort_by(|a, b| {
        let na = a.1.aps.parse::<f32>().unwrap_or(0.0);
        let nb = b.1.aps.parse::<f32>().unwrap_or(0.0);
        nb.partial_cmp(&na).unwrap().then(a.1.name.cmp(&b.1.name))
    });
    hits.dedup_by(|a, b| a.1.name == b.1.name);

    if hits.is_empty() {
        out.push_str("No listed 2027 programmes matched both the APS total and the subject ratings.\nTry another subject combination, or check a specific course name.");
        return out;
    }

    out.push_str("Courses you may qualify for (estimate from the 2027 prospectus):\n");
    for (aps, p, method) in hits.iter().take(15) {
        out.push_str(&format!("• {}  — needs APS {}, you have {:.1} ({})\n", p.name, p.aps, aps, method));
    }
    out.push_str("\nThis is a guide only. CPUT still checks interviews, portfolios, space, and official closing dates.");
    out
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn looks_like_aps_howto(q: &str) -> bool {
    let n = norm(q);
    (n.contains("how") && n.contains("aps"))
        || n.contains("calculat")
        || n.contains("admission point")
        || (n == "aps")
        || n.contains("how is aps")
}

#[allow(dead_code)]
pub fn sort_key(a: &ProgrammeAps, b: &ProgrammeAps) -> Ordering {
    a.name.cmp(&b.name)
}

use crate::aps::{format_programme, ProgrammeAps};

/// Detect whether the request came from the Course Information page.
pub fn looks_like_course_info(question: &str) -> bool {
    question
        .trim()
        .to_lowercase()
        .starts_with("course_info:")
}

/// Find a qualification using its actual programme name.
pub fn find_course<'a>(
    question: &str,
    programmes: &'a [ProgrammeAps],
) -> Option<&'a ProgrammeAps> {

    let search = clean_question(question);

    if search.is_empty() {
        return None;
    }

    // 1. Exact match first
    if let Some(programme) = programmes
        .iter()
        .find(|p| normalise(&p.name) == search)
    {
        return Some(programme);
    }

    // 2. Programme name contains the search
    if let Some(programme) = programmes
        .iter()
        .find(|p| normalise(&p.name).contains(&search))
    {
        return Some(programme);
    }

    // 3. Search contains the full programme name
    if let Some(programme) = programmes
        .iter()
        .find(|p| search.contains(&normalise(&p.name)))
    {
        return Some(programme);
    }

    None
}

/// Return admission information for the qualification.
pub fn course_information(
    question: &str,
    programmes: &[ProgrammeAps],
) -> String {

    match find_course(question, programmes) {

        Some(programme) => {

            let mut response = String::new();

            response.push_str("🎓 COURSE INFORMATION\n\n");

            response.push_str(
                &format_programme(programme)
            );

            response.push_str(
                "\n\nPlease remember: meeting the minimum \
admission requirements does not guarantee admission."
            );

            response
        }

        None => {

            let search = clean_question(question);

            format!(
                "I could not find a CPUT qualification matching \"{}\".\n\n\
Please check the qualification name and try again.\n\n\
Examples:\n\
• Diploma in Marketing\n\
• Diploma in Information and Communication Technology\n\
• Diploma in Analytical Chemistry\n\
• Bachelor of Nursing",
                search
            )
        }
    }
}

/// Remove the command inserted by chat.html.
fn clean_question(question: &str) -> String {
    let cleaned = question
        .trim()
        .strip_prefix("COURSE_INFO:")
        .unwrap_or(question)
        .trim();

    normalise(cleaned)
}

/// Normalise text for matching.
fn normalise(text: &str) -> String {

    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c.is_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
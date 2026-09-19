//! Page-accurate PDF extraction: paragraphs, printed-page offset, TOC hints.
//!
//! The worker persists one JSON file per page plus a manifest so later jobs
//! (chapter generation, citations, embeddings) never re-parse the PDF.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Soft cap for the TOC prompt sent to the model. The full book lives on S3;
/// Gemini only needs the contents pages (or a labelled front-matter fallback).
pub const TOC_PROMPT_MAX_BYTES: usize = 48_000;

/// How many labelled front-matter pages to include when no TOC is detected.
pub const TOC_FALLBACK_PAGES: usize = 40;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Paragraph {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtractedPage {
    pub pdf_page: u32,
    /// 1-based printed page once an Arabic offset is known; None for front matter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printed_page: Option<i32>,
    /// Raw label spotted on the page (`"iii"`, `"12"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub printed_label: Option<String>,
    pub paragraphs: Vec<Paragraph>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtractManifest {
    pub total_pages: u32,
    /// `printed_page = pdf_page as i32 - page_offset` when `printed_page >= 1`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_offset: Option<i32>,
    pub toc_pdf_pages: Vec<u32>,
    pub pages_prefix: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageLabel {
    Arabic(i32),
    Roman(i32),
}

pub fn page_object_key(prefix: &str, pdf_page: u32) -> String {
    format!(
        "{}/pages/{:04}.json",
        prefix.trim_end_matches('/'),
        pdf_page
    )
}

pub fn manifest_object_key(prefix: &str) -> String {
    format!("{}/manifest.json", prefix.trim_end_matches('/'))
}

pub fn extract_prefix(user_id: &str, subject_id: &str) -> String {
    format!("subjects/{user_id}/{subject_id}/extract")
}

/// `p12-2` → PDF page 12.
pub fn pdf_page_from_paragraph_id(id: &str) -> Option<u32> {
    let rest = id.strip_prefix('p')?;
    let (page, _) = rest.split_once('-')?;
    page.parse().ok()
}

/// Splits a page's extracted text into stable paragraph IDs (`p{page}-{n}`).
pub fn split_paragraphs(page_text: &str, pdf_page: u32) -> Vec<Paragraph> {
    let mut paragraphs = Vec::new();
    let mut current = String::new();

    for line in page_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            flush_paragraph(&mut current, &mut paragraphs, pdf_page);
            continue;
        }
        if is_page_number_only(line) {
            continue;
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(line);
    }
    flush_paragraph(&mut current, &mut paragraphs, pdf_page);

    if paragraphs.is_empty() {
        let text = normalize_ws(page_text);
        if !text.is_empty() && !is_page_number_only(&text) {
            paragraphs.push(Paragraph {
                id: format!("p{pdf_page}-1"),
                text,
            });
        }
    }

    paragraphs
}

fn flush_paragraph(current: &mut String, out: &mut Vec<Paragraph>, pdf_page: u32) {
    let text = normalize_ws(current);
    current.clear();
    if text.is_empty() || is_page_number_only(&text) {
        return;
    }
    let n = out.len() + 1;
    out.push(Paragraph {
        id: format!("p{pdf_page}-{n}"),
        text,
    });
}

fn normalize_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn is_page_number_only(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() {
        return false;
    }
    parse_page_label(t).is_some() && t.split_whitespace().count() == 1
}

pub fn parse_page_label(s: &str) -> Option<PageLabel> {
    let t = s.trim();
    if t.is_empty() || t.len() > 8 {
        return None;
    }
    if let Ok(n) = t.parse::<i32>()
        && (1..=2000).contains(&n)
    {
        return Some(PageLabel::Arabic(n));
    }
    parse_roman(&t.to_ascii_lowercase()).map(PageLabel::Roman)
}

/// Standard subtractive Roman numerals, limited to front-matter sizes.
pub fn parse_roman(s: &str) -> Option<i32> {
    if s.is_empty() || s.len() > 8 || !s.chars().all(|c| matches!(c, 'i' | 'v' | 'x' | 'l')) {
        return None;
    }
    let val = |c: char| match c {
        'i' => 1,
        'v' => 5,
        'x' => 10,
        'l' => 50,
        _ => 0,
    };
    let chars: Vec<char> = s.chars().collect();
    let mut total = 0;
    let mut i = 0;
    while i < chars.len() {
        let v = val(chars[i]);
        if i + 1 < chars.len() && v < val(chars[i + 1]) {
            total += val(chars[i + 1]) - v;
            i += 2;
        } else {
            total += v;
            i += 1;
        }
    }
    (1..=50).contains(&total).then_some(total)
}

/// Looks at the first and last few non-empty lines for a page number / folio.
pub fn detect_printed_label(page_text: &str) -> Option<String> {
    let lines: Vec<&str> = page_text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        return None;
    }

    let candidates = [
        lines.first().copied(),
        lines.last().copied(),
        lines.get(lines.len().saturating_sub(2)).copied(),
    ];

    for line in candidates.into_iter().flatten() {
        if let Some(label) = parse_page_label(line) {
            return Some(match label {
                PageLabel::Arabic(n) => n.to_string(),
                PageLabel::Roman(_) => line.to_ascii_lowercase(),
            });
        }
    }
    None
}

/// Votes for `offset` such that `printed = pdf_page - offset`. Needs 3 agreeing pages.
pub fn infer_page_offset(samples: &[(u32, Option<i32>)]) -> Option<i32> {
    let mut votes: HashMap<i32, u32> = HashMap::new();
    for (pdf_page, printed) in samples {
        let Some(printed) = printed else { continue };
        let offset = *pdf_page as i32 - printed;
        if (0..80).contains(&offset) {
            *votes.entry(offset).or_default() += 1;
        }
    }
    votes
        .into_iter()
        .filter(|(_, n)| *n >= 3)
        .max_by_key(|(_, n)| *n)
        .map(|(offset, _)| offset)
}

pub fn apply_page_offset(pages: &mut [ExtractedPage], offset: Option<i32>) {
    let Some(offset) = offset else { return };
    for page in pages {
        let printed = page.pdf_page as i32 - offset;
        page.printed_page = (printed >= 1).then_some(printed);
    }
}

/// Higher score = more likely a contents page.
pub fn toc_score(text: &str) -> i32 {
    let lower = text.to_ascii_lowercase();
    let mut score = 0;
    if lower.contains("table of contents") {
        score += 8;
    } else if lower.contains("contents") && text.len() < 6_000 {
        score += 3;
    }

    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.len() >= 5 {
        let short = lines.iter().filter(|l| l.len() < 80).count();
        if short * 100 / lines.len() > 70 {
            score += 2;
        }
        let ends_num = lines
            .iter()
            .filter(|l| {
                l.chars()
                    .rev()
                    .find(|c| !c.is_whitespace())
                    .is_some_and(|c| c.is_ascii_digit())
            })
            .count();
        if ends_num >= 5 {
            score += 3;
        }
        let chapterish = lines
            .iter()
            .filter(|l| {
                let l = l.to_ascii_lowercase();
                l.contains("chapter") || l.contains("part ") || l.starts_with("unit ")
            })
            .count();
        if chapterish >= 2 {
            score += 4;
        }
    }

    // Paragraph joining can collapse a contents list into one line; still count markers.
    let chapter_marks = lower.matches("chapter").count()
        + lower.matches("part ").count()
        + lower.matches("unit ").count();
    if chapter_marks >= 3 && score < 4 {
        score += 4;
    }
    score
}

pub fn detect_toc_pages(pages: &[ExtractedPage]) -> Vec<u32> {
    let scored: Vec<(u32, i32)> = pages
        .iter()
        .map(|p| {
            let text: String = p
                .paragraphs
                .iter()
                .map(|para| para.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            (p.pdf_page, toc_score(&text))
        })
        .collect();

    let hits: Vec<u32> = scored
        .iter()
        .filter(|(_, score)| *score >= 4)
        .map(|(page, _)| *page)
        .collect();

    if hits.is_empty() {
        return Vec::new();
    }

    let min = *hits.iter().min().unwrap();
    let max = *hits.iter().max().unwrap();
    // Keep a contiguous run so we don't skip a TOC page that scored just under the cut.
    (min..=max).filter(|p| *p - min < 12).collect()
}

fn truncate_to_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Builds the model prompt from TOC pages, or a labelled front-matter fallback.
pub fn build_toc_prompt(pages: &[ExtractedPage], manifest: &ExtractManifest) -> String {
    let mut header = format!("PDF pages: {}.\n", manifest.total_pages);
    if let Some(offset) = manifest.page_offset {
        header.push_str(&format!(
            "Printed-page offset: {offset} (printed_page = pdf_page - {offset}). Always return PDF page numbers as they appear in the 'PDF p.N' labels.\n"
        ));
    } else {
        header.push_str(
            "No printed-page offset was detected. page_start/page_end must be the PDF page numbers from the 'PDF p.N' labels.\n",
        );
    }

    let selected: Vec<&ExtractedPage> = if manifest.toc_pdf_pages.is_empty() {
        pages.iter().take(TOC_FALLBACK_PAGES).collect()
    } else {
        pages
            .iter()
            .filter(|p| manifest.toc_pdf_pages.contains(&p.pdf_page))
            .collect()
    };

    let mut body = String::new();
    for page in selected {
        let printed = match (page.printed_page, page.printed_label.as_deref()) {
            (Some(n), _) => format!(" / printed p.{n}"),
            (None, Some(label)) => format!(" / printed p.{label}"),
            _ => String::new(),
        };
        body.push_str(&format!("\n--- PDF p.{}{} ---\n", page.pdf_page, printed));
        for para in &page.paragraphs {
            body.push_str(&para.text);
            body.push('\n');
        }
    }

    let budget = TOC_PROMPT_MAX_BYTES.saturating_sub(header.len());
    format!("{header}{}", truncate_to_char_boundary(&body, budget))
}

pub fn page_from_text(pdf_page: u32, text: &str) -> ExtractedPage {
    ExtractedPage {
        pdf_page,
        printed_page: None,
        printed_label: detect_printed_label(text),
        paragraphs: split_paragraphs(text, pdf_page),
    }
}

pub fn arabic_from_label(label: Option<&str>) -> Option<i32> {
    match label.and_then(parse_page_label) {
        Some(PageLabel::Arabic(n)) => Some(n),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraph_ids_are_stable_per_page() {
        let text = "First thought.\n\nSecond thought.\n\n12";
        let paras = split_paragraphs(text, 17);
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[0].id, "p17-1");
        assert_eq!(paras[0].text, "First thought.");
        assert_eq!(paras[1].id, "p17-2");
        assert_eq!(pdf_page_from_paragraph_id("p17-2"), Some(17));
        assert_eq!(pdf_page_from_paragraph_id("bad"), None);
    }

    #[test]
    fn page_number_only_lines_are_dropped() {
        assert!(is_page_number_only("12"));
        assert!(is_page_number_only("iv"));
        assert!(!is_page_number_only("Chapter 12"));
        let paras = split_paragraphs("Hello world\n12\n", 3);
        assert_eq!(paras.len(), 1);
        assert_eq!(paras[0].text, "Hello world");
    }

    #[test]
    fn roman_parser_handles_front_matter() {
        assert_eq!(parse_roman("i"), Some(1));
        assert_eq!(parse_roman("iv"), Some(4));
        assert_eq!(parse_roman("xii"), Some(12));
        assert_eq!(parse_roman("xl"), Some(40));
        assert_eq!(parse_roman("hello"), None);
        assert_eq!(parse_roman(""), None);
    }

    #[test]
    fn detect_label_from_folio() {
        let text = "A long paragraph about databases.\n\n17";
        assert_eq!(detect_printed_label(text).as_deref(), Some("17"));
        let text = "iii\nPreface goes here.";
        assert_eq!(detect_printed_label(text).as_deref(), Some("iii"));
    }

    #[test]
    fn offset_needs_three_agreeing_arabic_pages() {
        let samples = [(17, Some(1)), (18, Some(2)), (19, Some(3)), (5, Some(99))];
        assert_eq!(infer_page_offset(&samples), Some(16));
        assert_eq!(infer_page_offset(&[(1, Some(1)), (2, Some(2))]), None);
    }

    #[test]
    fn apply_offset_skips_front_matter() {
        let mut pages = vec![
            page_from_text(1, "iii\nPreface"),
            page_from_text(17, "Chapter one.\n1"),
        ];
        apply_page_offset(&mut pages, Some(16));
        assert_eq!(pages[0].printed_page, None);
        assert_eq!(pages[1].printed_page, Some(1));
    }

    #[test]
    fn toc_pages_form_a_contiguous_run() {
        let pages = vec![
            page_from_text(1, "Title page of the book"),
            page_from_text(
                5,
                "Table of Contents\nChapter 1 ........ 1\nChapter 2 ........ 20\nChapter 3 ........ 40\nChapter 4 ........ 66\nChapter 5 ........ 80\nChapter 6 ........ 99\nChapter 7 ........ 120\nChapter 8 ........ 140",
            ),
            page_from_text(
                6,
                "Chapter 9 ........ 160\nChapter 10 ....... 180\nPart II .......... 200\nChapter 11 ....... 220\nChapter 12 ....... 240",
            ),
            page_from_text(20, "Once upon a time the real chapter started."),
        ];
        let toc = detect_toc_pages(&pages);
        assert!(toc.contains(&5));
        assert!(toc.contains(&6));
        assert!(!toc.contains(&20));
    }

    #[test]
    fn toc_prompt_stays_within_budget_and_labels_pages() {
        let pages: Vec<ExtractedPage> = (1..=5)
            .map(|n| page_from_text(n, &format!("Contents line {n}\nChapter {n} ...... {n}")))
            .collect();
        let manifest = ExtractManifest {
            total_pages: 200,
            page_offset: Some(4),
            toc_pdf_pages: vec![1, 2],
            pages_prefix: "subjects/u/s/extract".into(),
        };
        let prompt = build_toc_prompt(&pages, &manifest);
        assert!(prompt.contains("PDF pages: 200"));
        assert!(prompt.contains("PDF p.1"));
        assert!(prompt.contains("Printed-page offset: 4"));
        assert!(prompt.len() <= TOC_PROMPT_MAX_BYTES);
        assert!(!prompt.contains("Contents line 5"));
    }

    #[test]
    fn object_keys_are_zero_padded() {
        assert_eq!(
            page_object_key("subjects/u/s/extract", 7),
            "subjects/u/s/extract/pages/0007.json"
        );
        assert_eq!(
            manifest_object_key("subjects/u/s/extract/"),
            "subjects/u/s/extract/manifest.json"
        );
    }
}

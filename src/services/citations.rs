//! Verify model citations against the page extract, and retrieve paragraphs
//! for grounded scene planning. Quotes that do not appear on the claimed
//! page/paragraph are dropped — never shown to the student.

use crate::models::{ChapterPlan, Citation, SceneSpec, retain_grounded_citations};
use crate::services::extract::{ExtractedPage, Paragraph};
use std::collections::{HashMap, HashSet};

const MIN_QUOTE_CHARS: usize = 8;
const CATALOG_PREVIEW_CHARS: usize = 180;
const FALLBACK_CITATIONS: usize = 2;

const STOPWORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "of", "to", "in", "on", "for", "is", "are", "this", "that",
    "with", "from", "as", "at", "by", "it", "be", "was", "were", "its", "into", "than", "then",
    "they", "their", "them", "we", "you", "your", "can", "may", "not", "but", "if", "when",
];

#[derive(Debug, Clone)]
pub struct IndexedParagraph {
    pub id: String,
    pub pdf_page: i32,
    pub printed_page: Option<i32>,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct ParagraphIndex {
    by_id: HashMap<String, IndexedParagraph>,
    by_page: HashMap<i32, Vec<String>>,
}

impl ParagraphIndex {
    pub fn from_pages(pages: &[ExtractedPage]) -> Self {
        let mut index = Self::default();
        for page in pages {
            let pdf_page = page.pdf_page as i32;
            for para in &page.paragraphs {
                let id = para.id.clone();
                index.by_page.entry(pdf_page).or_default().push(id.clone());
                index.by_id.insert(
                    id,
                    IndexedParagraph {
                        id: para.id.clone(),
                        pdf_page,
                        printed_page: page.printed_page,
                        text: para.text.clone(),
                    },
                );
            }
        }
        index
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub fn get(&self, id: &str) -> Option<&IndexedParagraph> {
        self.by_id.get(id)
    }

    pub fn paragraphs_on_page(&self, pdf_page: i32) -> Vec<&IndexedParagraph> {
        self.by_page
            .get(&pdf_page)
            .into_iter()
            .flatten()
            .filter_map(|id| self.by_id.get(id))
            .collect()
    }

    pub fn all(&self) -> impl Iterator<Item = &IndexedParagraph> {
        self.by_id.values()
    }

    /// Compact catalog the planner uses to pick paragraph IDs.
    pub fn catalog(&self) -> String {
        let mut ids: Vec<&String> = self.by_id.keys().collect();
        ids.sort();
        let mut lines = Vec::with_capacity(ids.len());
        for id in ids {
            let Some(p) = self.by_id.get(id) else {
                continue;
            };
            let preview = truncate_chars(&p.text, CATALOG_PREVIEW_CHARS);
            let printed = match p.printed_page {
                Some(n) => format!("printed p.{n}"),
                None => "printed unknown".into(),
            };
            lines.push(format!(
                "[{}] PDF p.{} / {} — {preview}",
                p.id, p.pdf_page, printed
            ));
        }
        lines.join("\n")
    }
}

pub fn prompt_from_pages(pages: &[ExtractedPage]) -> String {
    let mut parts = Vec::new();
    for page in pages {
        let printed = match page.printed_page {
            Some(n) => format!("printed p.{n}"),
            None => page
                .printed_label
                .as_deref()
                .map(|l| format!("printed p.{l}"))
                .unwrap_or_else(|| "printed unknown".into()),
        };
        parts.push(format!("--- PDF p.{} / {} ---", page.pdf_page, printed));
        for para in &page.paragraphs {
            parts.push(format!("[{}] {}", para.id, para.text));
        }
    }
    parts.join("\n")
}

/// Builds the generate prompt from a planner result: only the retrieved
/// paragraphs, grouped by scene. Unknown IDs are replaced by lexical hits.
pub fn prompt_from_plan(plan: &ChapterPlan, index: &ParagraphIndex) -> String {
    let mut parts = Vec::new();
    if !plan.kid_lede.is_empty() {
        parts.push(format!("Chapter hook: {}", plan.kid_lede));
    }
    for scene in &plan.scenes {
        let mut ids = scene
            .paragraph_ids
            .iter()
            .filter(|id| index.get(id).is_some())
            .cloned()
            .collect::<Vec<_>>();
        if ids.is_empty() {
            let query = format!("{} {}", scene.title, scene.focus);
            ids = retrieve_paragraphs(&query, index, 4)
                .into_iter()
                .map(|p| p.id.clone())
                .collect();
        }
        if ids.is_empty() {
            continue;
        }
        parts.push(format!(
            "## Scene: {} ({})\nFocus: {}\nUse only these paragraphs:",
            scene.title, scene.id, scene.focus
        ));
        parts.push(prompt_from_ids(&ids, index));
    }
    parts.join("\n\n")
}

pub fn prompt_from_ids(ids: &[String], index: &ParagraphIndex) -> String {
    let mut parts = Vec::new();
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id.clone()) {
            continue;
        }
        let Some(p) = index.get(id) else {
            continue;
        };
        let printed = match p.printed_page {
            Some(n) => format!("printed p.{n}"),
            None => "printed unknown".into(),
        };
        parts.push(format!(
            "[{}] PDF p.{} / {} — {}",
            p.id, p.pdf_page, printed, p.text
        ));
    }
    parts.join("\n")
}

pub fn collapsed_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// If `quote` appears in `paragraph` (whitespace-insensitive), return a
/// collapsed quote taken from the book — not the model's possibly messy copy.
pub fn extract_matching_quote(paragraph: &str, quote: &str) -> Option<String> {
    let hay = collapsed_ws(paragraph);
    let needle = collapsed_ws(quote);
    if needle.len() < MIN_QUOTE_CHARS || !hay.contains(&needle) {
        return None;
    }
    Some(needle)
}

/// Accepts a citation only when the quote appears on the claimed paragraph
/// or, failing that, on the claimed PDF page. Page and printed folio are
/// rewritten from the extract so the player never shows a hallucinated page.
pub fn verify_citation(cite: Citation, index: &ParagraphIndex) -> Option<Citation> {
    if let Some(p) = index.get(&cite.paragraph_id)
        && let Some(quote) = extract_matching_quote(&p.text, &cite.quote)
    {
        return Some(bind_citation(p, quote));
    }

    for p in index.paragraphs_on_page(cite.pdf_page) {
        if let Some(quote) = extract_matching_quote(&p.text, &cite.quote) {
            return Some(bind_citation(p, quote));
        }
    }

    None
}

fn bind_citation(p: &IndexedParagraph, quote: String) -> Citation {
    Citation {
        pdf_page: p.pdf_page,
        printed_page: p.printed_page,
        paragraph_id: p.id.clone(),
        quote,
    }
}

pub fn verify_citations(citations: Vec<Citation>, index: &ParagraphIndex) -> Vec<Citation> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for cite in citations {
        let Some(ok) = verify_citation(cite, index) else {
            continue;
        };
        if seen.insert(ok.paragraph_id.clone()) {
            out.push(ok);
        }
    }
    out
}

fn tokens(s: &str) -> HashSet<String> {
    s.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() > 2 && !STOPWORDS.contains(w))
        .map(str::to_string)
        .collect()
}

/// Rank paragraphs in this chapter against a query (scene title + summary).
pub fn retrieve_paragraphs<'a>(
    query: &str,
    index: &'a ParagraphIndex,
    k: usize,
) -> Vec<&'a IndexedParagraph> {
    let q = tokens(query);
    if q.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(&IndexedParagraph, f32)> = index
        .all()
        .map(|p| {
            let pt = tokens(&p.text);
            let overlap = q.intersection(&pt).count() as f32;
            let denom = (q.len() as f32).sqrt().max(1.0);
            (p, overlap / denom)
        })
        .filter(|(_, score)| *score > 0.0)
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().take(k).map(|(p, _)| p).collect()
}

pub fn first_quotable_span(text: &str) -> Option<String> {
    let collapsed = collapsed_ws(text);
    if collapsed.len() < MIN_QUOTE_CHARS {
        return None;
    }
    let sentence = collapsed
        .split_once(". ")
        .map(|(head, _)| {
            if head.len() >= MIN_QUOTE_CHARS {
                format!("{head}.")
            } else {
                collapsed.clone()
            }
        })
        .unwrap_or(collapsed);
    Some(truncate_chars(&sentence, 220))
}

/// When the model cites nothing usable, attach the closest extract paragraphs
/// so the scene still has a real page number for the student.
pub fn fallback_citations(query: &str, index: &ParagraphIndex, k: usize) -> Vec<Citation> {
    retrieve_paragraphs(query, index, k)
        .into_iter()
        .filter_map(|p| first_quotable_span(&p.text).map(|quote| bind_citation(p, quote)))
        .collect()
}

/// Drops unverified quotes, fills page metadata from the extract, and
/// attaches fallback citations so every remaining scene is grounded.
/// Scenes that still have no citation are removed.
pub fn ground_scenes(scenes: &mut Vec<SceneSpec>, index: &ParagraphIndex) -> GroundingStats {
    let mut stats = GroundingStats::default();
    for scene in scenes.iter_mut() {
        let incoming = std::mem::take(&mut scene.citations);
        stats.incoming += incoming.len();
        scene.citations = verify_citations(incoming, index);
        stats.verified += scene.citations.len();
        if scene.citations.is_empty() {
            let query = format!("{} {} {}", scene.title, scene.kid_summary, scene.depth.text);
            scene.citations = fallback_citations(&query, index, FALLBACK_CITATIONS);
            stats.repaired += scene.citations.len();
        }
    }
    let before = scenes.len();
    scenes.retain(|s| !s.citations.is_empty());
    stats.dropped_scenes = before - scenes.len();
    stats
}

/// Fallback when no extract index exists (DOCX / missing pages): keep quotes
/// that appear in the raw chapter text.
pub fn ground_scenes_against_text(scenes: &mut Vec<SceneSpec>, text: &str) {
    for scene in scenes.iter_mut() {
        let incoming = std::mem::take(&mut scene.citations);
        scene.citations = retain_grounded_citations(text, incoming);
    }
    scenes.retain(|s| !s.citations.is_empty());
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GroundingStats {
    pub incoming: usize,
    pub verified: usize,
    pub repaired: usize,
    pub dropped_scenes: usize,
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Helper for tests and extract→index fixtures.
pub fn page(pdf_page: u32, printed: Option<i32>, paragraphs: Vec<(&str, &str)>) -> ExtractedPage {
    ExtractedPage {
        pdf_page,
        printed_page: printed,
        printed_label: printed.map(|n| n.to_string()),
        paragraphs: paragraphs
            .into_iter()
            .map(|(id, text)| Paragraph {
                id: id.to_string(),
                text: text.to_string(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{SceneDepth, VizSpec};

    fn sample_index() -> ParagraphIndex {
        ParagraphIndex::from_pages(&[
            page(
                14,
                Some(3),
                vec![(
                    "p14-2",
                    "A packet is a small labeled box of data. It has a header and a payload.",
                )],
            ),
            page(
                15,
                Some(4),
                vec![(
                    "p15-1",
                    "Routers forward each packet toward its destination.",
                )],
            ),
        ])
    }

    fn cite(page: i32, id: &str, quote: &str) -> Citation {
        Citation {
            pdf_page: page,
            printed_page: None,
            paragraph_id: id.into(),
            quote: quote.into(),
        }
    }

    #[test]
    fn accepts_quote_on_claimed_paragraph() {
        let index = sample_index();
        let ok = verify_citation(
            cite(14, "p14-2", "A packet is a small labeled box of data."),
            &index,
        )
        .expect("verified");
        assert_eq!(ok.paragraph_id, "p14-2");
        assert_eq!(ok.pdf_page, 14);
        assert_eq!(ok.printed_page, Some(3));
    }

    #[test]
    fn fills_printed_page_from_extract() {
        let index = sample_index();
        let ok = verify_citation(
            Citation {
                pdf_page: 14,
                printed_page: Some(99),
                paragraph_id: "p14-2".into(),
                quote: "A packet is a small labeled box of data.".into(),
            },
            &index,
        )
        .expect("verified");
        assert_eq!(ok.printed_page, Some(3));
    }

    #[test]
    fn rejects_quote_not_on_claimed_page() {
        let index = sample_index();
        assert!(
            verify_citation(
                cite(99, "p99-1", "A packet is a small labeled box of data."),
                &index
            )
            .is_none()
        );
    }

    #[test]
    fn rebinds_wrong_id_when_quote_is_on_claimed_page() {
        let index = sample_index();
        let ok = verify_citation(
            cite(14, "p14-99", "A packet is a small labeled box of data."),
            &index,
        )
        .expect("rebound");
        assert_eq!(ok.paragraph_id, "p14-2");
    }

    #[test]
    fn rejects_invented_quote() {
        let index = sample_index();
        assert!(
            verify_citation(
                cite(14, "p14-2", "Packets travel faster than light."),
                &index
            )
            .is_none()
        );
    }

    #[test]
    fn quote_match_ignores_whitespace() {
        let index = sample_index();
        let ok = verify_citation(
            cite(14, "p14-2", "A packet   is a small\nlabeled box of data."),
            &index,
        );
        assert!(ok.is_some());
    }

    #[test]
    fn retrieve_ranks_overlapping_paragraph() {
        let index = sample_index();
        let hits = retrieve_paragraphs("how routers forward a packet", &index, 2);
        assert_eq!(hits[0].id, "p15-1");
    }

    #[test]
    fn fallback_attaches_real_page_numbers() {
        let index = sample_index();
        let cites = fallback_citations("routers forward packets", &index, 1);
        assert_eq!(cites.len(), 1);
        assert_eq!(cites[0].paragraph_id, "p15-1");
        assert_eq!(cites[0].printed_page, Some(4));
    }

    #[test]
    fn ground_scenes_drops_uncitable_and_repairs_empty() {
        let index = sample_index();
        let scene = |title: &str, cites: Vec<Citation>| SceneSpec {
            id: title.to_string(),
            title: title.into(),
            kid_summary: title.into(),
            viz: VizSpec {
                kind: "steps".into(),
                params: serde_json::json!({}),
            },
            depth: SceneDepth {
                text: title.into(),
                audio_key: None,
                audio_url: None,
            },
            citations: cites,
            quiz: None,
        };
        let mut scenes = vec![
            scene(
                "Packets",
                vec![cite(
                    14,
                    "p14-2",
                    "A packet is a small labeled box of data.",
                )],
            ),
            scene(
                "Made up",
                vec![cite(99, "p99-1", "This sentence is not in the book.")],
            ),
        ];
        // Second scene query tokens won't match well... "Made up" may not retrieve.
        // Give it router words so fallback can attach.
        scenes[1].kid_summary = "Routers forward each packet".into();
        scenes[1].depth.text = "Routers forward each packet toward its destination.".into();
        let stats = ground_scenes(&mut scenes, &index);
        assert_eq!(scenes.len(), 2);
        assert_eq!(stats.verified, 1);
        assert!(stats.repaired >= 1);
        assert_eq!(scenes[0].citations[0].printed_page, Some(3));
        assert_eq!(scenes[1].citations[0].paragraph_id, "p15-1");
        assert!(
            scenes
                .iter()
                .flat_map(|s| &s.citations)
                .all(|c| c.quote != "This sentence is not in the book.")
        );
    }

    #[test]
    fn prompt_from_plan_uses_only_retrieved_paragraphs() {
        let index = sample_index();
        let plan = ChapterPlan {
            kid_lede: "Hook".into(),
            scenes: vec![crate::models::ScenePlan {
                id: "packets".into(),
                title: "Packets".into(),
                focus: "labeled box".into(),
                paragraph_ids: vec!["p14-2".into(), "p99-1".into()],
            }],
        };
        let prompt = prompt_from_plan(&plan, &index);
        assert!(prompt.contains("[p14-2]"));
        assert!(prompt.contains("A packet is a small labeled box"));
        assert!(!prompt.contains("p99-1"));
        assert!(!prompt.contains("Routers forward"));
    }

    #[test]
    fn catalog_lists_ids_and_previews() {
        let catalog = sample_index().catalog();
        assert!(catalog.contains("[p14-2]"));
        assert!(catalog.contains("PDF p.14"));
        assert!(catalog.contains("A packet is a small"));
    }
}

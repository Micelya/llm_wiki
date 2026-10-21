//! The search of the base: the pages of the documents it finds and of
//! whom the question names, for the search screen and the chat's agent
//! (D-408, D-413), and the tokenizer the base shares.

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::commands::search_spanish;

const MAX_RESULTS: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchImageRef {
    pub url: String,
    pub alt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSearchResult {
    pub path: String,
    pub title: String,
    pub snippet: String,
    pub title_match: bool,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_score: Option<f32>,
    pub images: Vec<SearchImageRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub graph_related_to: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSearchResponse {
    pub mode: String,
    pub results: Vec<ProjectSearchResult>,
    pub token_hits: usize,
    pub vector_hits: usize,
    pub graph_hits: usize,
}

/// Pages of entities the question names that the results carry, at most
/// (the most specific first): the documents keep the other places. One
/// page costs one document of 104 in the questions of the test folder and
/// brings what the record knows of the entity (D-352).
const NAMED_ENTITY_PAGES: usize = 1;

/// The search of the base (D-350, D-408): it searches the text, numbers
/// and summaries of every visible document, and the graph of the base
/// orders and widens what it finds without taking anything out.
/// Each document is answered with its page (versions share one, D-340);
/// the person, organization or matter the question names most
/// specifically adds its page after the documents. `None` while no document has a page yet.
fn search_from_base(
    base: &crate::docbase::DocBase,
    query: &str,
    limit: usize,
    include_content: bool,
) -> Result<Option<ProjectSearchResponse>, String> {
    let pages = base.document_pages()?;
    if pages.is_empty() {
        return Ok(None);
    }
    // Every screen of the private space sees the whole base today; the
    // shared space will pass the set of each person here.
    let visible = crate::docbase::Visible::All;
    let found = base.search_documents(&visible, query, (limit * 4).max(40), None)?;
    let entity_pages = base.entity_pages(&found.named)?;
    let entity_slots = entity_pages.len().min(NAMED_ENTITY_PAGES).min(limit.saturating_sub(1));
    let mut results: Vec<ProjectSearchResult> = Vec::new();
    let mut seen = BTreeSet::new();
    let related_names = |entities: &[String]| -> Vec<String> {
        entities.iter().filter_map(|id| found.names.get(id).cloned()).collect()
    };
    for hit in &found.hits {
        if results.len() >= limit - entity_slots {
            break;
        }
        let Some(path) = pages.get(&hit.content_id) else { continue };
        if !seen.insert(normalize_path(path)) {
            continue;
        }
        results.push(page_result(base, path, &hit.snippet, hit.score, include_content, related_names(&hit.entities)));
    }
    let lowest = results.last().map(|r| r.score).unwrap_or(1.0);
    for id in &found.named {
        if results.len() >= limit {
            break;
        }
        let Some(path) = entity_pages.get(id) else { continue };
        if !seen.insert(normalize_path(path)) {
            continue;
        }
        let name = found.names.get(id).cloned().unwrap_or_default();
        results.push(page_result(base, path, "", lowest * 0.99, include_content, vec![name]));
    }
    let graph_hits = results.iter().filter(|r| !r.graph_related_to.is_empty()).count();
    Ok(Some(ProjectSearchResponse {
        mode: "base".to_string(),
        token_hits: found.hits.len(),
        vector_hits: 0,
        graph_hits,
        results,
    }))
}

/// The search of the base for the search screen, with no project: the
/// pages of the documents it finds and of whom the question names (D-408).
pub fn search_base(base: &crate::docbase::DocBase, query: &str, top_k: usize) -> Result<ProjectSearchResponse, String> {
    if query.trim().is_empty() {
        return Err("query is required".to_string());
    }
    let limit = top_k.clamp(1, MAX_RESULTS);
    Ok(search_from_base(base, query, limit, false)?.unwrap_or_else(|| ProjectSearchResponse {
        mode: "base".to_string(),
        results: Vec::new(),
        token_hits: 0,
        vector_hits: 0,
        graph_hits: 0,
    }))
}

/// A page of the base as a result: its title and text from the base, not
/// from a file (D-402).
fn page_result(
    base: &crate::docbase::DocBase,
    path: &str,
    snippet: &str,
    score: f64,
    include_content: bool,
    related: Vec<String>,
) -> ProjectSearchResult {
    let (stored_title, content) = base.knowledge_page_text(path).ok().flatten().unwrap_or_default();
    let file_name = Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
    let title = Some(stored_title)
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| extract_title(&content, &file_name));
    ProjectSearchResult {
        path: path.to_string(),
        title,
        snippet: snippet.to_string(),
        title_match: false,
        score,
        vector_score: None,
        images: extract_image_refs(&content),
        content: include_content.then_some(content),
        graph_related_to: related,
    }
}

pub fn tokenize_query(query: &str) -> Vec<String> {
    let raw = query
        .to_lowercase()
        .split(is_query_separator)
        .filter(|token| token.chars().count() > 1)
        .filter(|token| !is_stop_word(token))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    let mut out = Vec::new();
    for token in raw {
        let chars = token.chars().collect::<Vec<_>>();
        let has_cjk = chars.iter().any(|c| ('\u{3400}'..='\u{9fff}').contains(c));
        if has_cjk && chars.len() > 2 {
            for pair in chars.windows(2) {
                out.push(pair.iter().collect());
            }
            for ch in &chars {
                let s = ch.to_string();
                if !is_stop_word(&s) {
                    out.push(s);
                }
            }
            out.push(token);
        } else {
            out.push(token);
        }
    }
    out.into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn is_query_separator(c: char) -> bool {
    c.is_whitespace()
        || c.is_ascii_punctuation()
        || search_spanish::is_separator(c)
        || matches!(
            c,
            '，' | '。'
                | '！'
                | '？'
                | '、'
                | '；'
                | '：'
                | '“'
                | '”'
                | '‘'
                | '’'
                | '（'
                | '）'
                | '·'
                | '～'
                | '…'
        )
}

fn is_stop_word(token: &str) -> bool {
    search_spanish::is_stop_word(token) || matches!(
        token,
        "的" | "是"
            | "了"
            | "什么"
            | "在"
            | "有"
            | "和"
            | "与"
            | "对"
            | "从"
            | "the"
            | "is"
            | "a"
            | "an"
            | "what"
            | "how"
            | "are"
            | "was"
            | "were"
            | "do"
            | "does"
            | "did"
            | "be"
            | "been"
            | "being"
            | "have"
            | "has"
            | "had"
            | "it"
            | "its"
            | "in"
            | "on"
            | "at"
            | "to"
            | "for"
            | "of"
            | "with"
            | "by"
            | "this"
            | "that"
            | "these"
            | "those"
    )
}

pub fn extract_title(content: &str, file_name: &str) -> String {
    let has_frontmatter = content.starts_with("---");
    let mut in_frontmatter = has_frontmatter;
    let mut frontmatter_closed = false;
    for line in content.lines().skip(if has_frontmatter { 1 } else { 0 }) {
        let trimmed = line.trim();
        if in_frontmatter && trimmed == "---" {
            in_frontmatter = false;
            frontmatter_closed = true;
            continue;
        }
        if in_frontmatter && trimmed.starts_with("title:") {
            return trimmed
                .trim_start_matches("title:")
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
        }
        if has_frontmatter && !frontmatter_closed {
            continue;
        }
        if let Some(title) = trimmed.strip_prefix("# ") {
            return title.trim().to_string();
        }
    }
    file_name.trim_end_matches(".md").replace('-', " ")
}

pub fn extract_image_refs(content: &str) -> Vec<SearchImageRef> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let mut rest = content;
    while let Some(start) = rest.find("![") {
        rest = &rest[start + 2..];
        let Some(alt_end) = rest.find("](") else {
            break;
        };
        let alt = &rest[..alt_end];
        rest = &rest[alt_end + 2..];
        let Some(url_end) = rest.find(')') else {
            break;
        };
        let url = &rest[..url_end];
        if !url.trim().is_empty()
            && !url.contains(char::is_whitespace)
            && seen.insert(url.to_string())
        {
            out.push(SearchImageRef {
                url: url.to_string(),
                alt: alt.to_string(),
            });
        }
        rest = &rest[url_end + 1..];
    }
    out
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::fs;

    #[test]
    fn the_search_of_the_base_needs_no_project() {
        let base = crate::docbase::DocBase::in_memory().unwrap();
        assert!(search_base(&base, "  ", 10).is_err());
        // No page yet: nothing found.
        let found = search_base(&base, "contrato", 10).unwrap();
        assert_eq!(found.mode, "base");
        assert!(found.results.is_empty());
    }

    #[test]
    fn tokenizes_cjk_bigrams_and_chars() {
        let tokens = tokenize_query("默会知识");
        assert!(tokens.contains(&"默会".to_string()));
        assert!(tokens.contains(&"知识".to_string()));
        assert!(tokens.contains(&"默".to_string()));
    }

    #[test]
    fn extracts_image_refs_without_duplicates() {
        let refs = extract_image_refs("![a](media/x.png)\n![b](media/x.png)");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].alt, "a");
    }

    #[test]
    fn extract_title_uses_frontmatter_or_heading_not_body_title_lines() {
        let with_frontmatter = "---\ntitle: Real Title\n---\n\ntitle: Body Label\n# Heading";
        assert_eq!(
            extract_title(with_frontmatter, "fallback-name.md"),
            "Real Title"
        );

        let without_frontmatter = "intro\ntitle: Body Label\n# Real Heading";
        assert_eq!(
            extract_title(without_frontmatter, "fallback-name.md"),
            "Real Heading"
        );

        assert_eq!(
            extract_title("plain body", "vector-database.md"),
            "vector database"
        );
    }

    /// Runs the search of the base the chat uses over a list of questions
    /// and writes what it returned, so a harness outside the repository can
    /// measure which documents each question reaches. Ignored: it needs a
    /// copy of a base and a questions file, both outside the repository.
    ///
    /// - `MICELYA_SEARCH_QUESTIONS`: JSON with `preguntas: [{id, pregunta}]`.
    /// - `MICELYA_SEARCH_BASE`: a COPY of a base.
    /// - `MICELYA_SEARCH_OUT`: where to write the results (JSON).
    /// - `MICELYA_SEARCH_TOPK`: results per question (default 10, as the chat asks).
    #[test]
    #[ignore]
    fn search_questions_from_env() {
        let questions_path =
            std::env::var("MICELYA_SEARCH_QUESTIONS").expect("MICELYA_SEARCH_QUESTIONS");
        let base_path = std::env::var("MICELYA_SEARCH_BASE").expect("MICELYA_SEARCH_BASE");
        let out_path = std::env::var("MICELYA_SEARCH_OUT").expect("MICELYA_SEARCH_OUT");
        let top_k = std::env::var("MICELYA_SEARCH_TOPK")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(10);
        let base = crate::docbase::DocBase::open(Path::new(&base_path)).unwrap();
        let sheet: Value =
            serde_json::from_str(&fs::read_to_string(&questions_path).unwrap()).unwrap();
        let mut rows = Vec::new();
        for question in sheet["preguntas"].as_array().expect("preguntas") {
            let id = question["id"].as_str().unwrap_or_default().to_string();
            let text = question["pregunta"].as_str().unwrap_or_default().to_string();
            let start = std::time::Instant::now();
            let response = search_base(&base, &text, top_k);
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            rows.push(match response {
                Ok(found) => json!({
                    "id": id,
                    "ms": ms,
                    "mode": found.mode,
                    "results": found
                        .results
                        .iter()
                        .map(|result| json!({
                            "path": result.path,
                            "title": result.title,
                            "score": result.score,
                            "graph": !result.graph_related_to.is_empty(),
                        }))
                        .collect::<Vec<_>>(),
                }),
                Err(error) => json!({ "id": id, "ms": ms, "error": error }),
            });
        }
        let report = json!({ "top_k": top_k, "rows": rows });
        fs::write(&out_path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }
}

//! A page of a portal as the app reads it.
//!
//! The view returns the page's HTML (`scripts::snapshot`); here it becomes
//! a flat list of elements in document order, each with its tag, its
//! attributes, its text and where its subtree ends. Connectors read the
//! page by its visible labels, never by the ids the portal generates.
//!
//! The elements the app may act on carry a mark (`data-micelya`) that the
//! snapshot put on the live page: clicking or filling one is asking the
//! view for the element with that mark, so what is read here and what is
//! touched there are always the same element.

use dom_query::{Document, NodeRef};
use serde::Deserialize;

/// Attribute the snapshot puts on the elements the app may act on.
pub const MARK: &str = "data-micelya";
/// Attribute the snapshot puts on the elements the person cannot see.
pub const HIDDEN: &str = "data-micelya-hidden";
/// Attribute the snapshot puts on each field with what it says now.
pub const VALUE: &str = "data-micelya-value";

/// What the view returns when asked for the page.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Snapshot {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub ready: String,
    #[serde(default)]
    pub html: String,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct El {
    pub tag: String,
    attrs: Vec<(String, String)>,
    /// Its text, with the spaces collapsed (what a person reads).
    pub text: String,
    pub parent: Option<usize>,
    /// Index of its last descendant (itself when it has none).
    pub end: usize,
    hidden: bool,
}

impl El {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_str())
    }

    /// Its mark on the live page, when the snapshot gave it one.
    pub fn mark(&self) -> Option<&str> {
        self.attr(MARK)
    }

    /// Its type, for an input ("text" when it says none).
    pub fn input_type(&self) -> String {
        self.attr("type").map(|t| t.trim().to_lowercase()).unwrap_or_else(|| "text".to_string())
    }

    /// What a person reads on it: its text, or the value of a button.
    pub fn label(&self) -> String {
        if self.tag == "input" {
            return collapse(self.attr("value").unwrap_or(""));
        }
        self.text.clone()
    }

    /// Its name for a person when it has no text: a link made of an icon
    /// is named by its `title` (or `aria-label`).
    pub fn title(&self) -> String {
        collapse(self.attr("title").or_else(|| self.attr("aria-label")).unwrap_or(""))
    }
}

#[derive(Debug, Clone, Default)]
pub struct Page {
    pub url: String,
    pub els: Vec<El>,
}

/// Tags whose text a person does not read.
const NO_TEXT: &[&str] = &["script", "style", "noscript", "template", "head", "title"];

/// Tags that separate words: their text never glues to the next one.
const BLOCKS: &[&str] = &[
    "address", "article", "aside", "blockquote", "br", "dd", "div", "dl", "dt", "fieldset", "footer", "form",
    "h1", "h2", "h3", "h4", "h5", "h6", "header", "hr", "label", "legend", "li", "main", "nav", "ol", "option",
    "p", "section", "table", "tbody", "td", "tfoot", "th", "thead", "tr", "ul",
];

impl Page {
    pub fn parse(url: &str, html: &str) -> Page {
        let document = Document::from(html);
        let mut els = Vec::new();
        collect(&document.root(), None, &mut els, false);
        Page { url: url.to_string(), els }
    }

    pub fn len(&self) -> usize {
        self.els.len()
    }

    pub fn el(&self, index: usize) -> &El {
        &self.els[index]
    }

    /// Whether `inner` is inside `outer` (or is it).
    pub fn contains(&self, outer: usize, inner: usize) -> bool {
        outer <= inner && inner <= self.els[outer].end
    }

    /// Its element children, in order.
    pub fn children(&self, index: usize) -> Vec<usize> {
        (index + 1..=self.els[index].end).filter(|&j| self.els[j].parent == Some(index)).collect()
    }

    /// Itself or its closest ancestor with one of these tags.
    pub fn closest(&self, index: usize, tags: &[&str]) -> Option<usize> {
        let mut current = Some(index);
        while let Some(i) = current {
            if tags.contains(&self.els[i].tag.as_str()) {
                return Some(i);
            }
            current = self.els[i].parent;
        }
        None
    }

    /// Its closest ancestor (not itself) with one of these tags.
    pub fn ancestor(&self, index: usize, tags: &[&str]) -> Option<usize> {
        self.els[index].parent.and_then(|parent| self.closest(parent, tags))
    }

    /// Whether a person can see it: neither it nor an ancestor is hidden.
    pub fn visible(&self, index: usize) -> bool {
        let mut current = Some(index);
        while let Some(i) = current {
            if self.els[i].hidden {
                return false;
            }
            current = self.els[i].parent;
        }
        true
    }

    /// The element a click on this one acts on: itself or its closest
    /// link or button, when the snapshot marked it.
    pub fn clickable(&self, index: usize) -> Option<usize> {
        let target = self.closest(index, &["a", "button", "input"])?;
        self.els[target].mark().map(|_| target)
    }

    /// The deepest visible elements whose words, normalized, are one of
    /// `wanted`: a label as a person reads it ("Jurisdicción:").
    pub fn labels(&self, wanted: &[&str], range: std::ops::Range<usize>) -> Vec<usize> {
        let matches = |i: usize| {
            let el = &self.els[i];
            el.text.chars().count() <= 60 && wanted.contains(&label_norm(&el.label()).as_str())
        };
        range
            .filter(|&i| i < self.els.len() && self.visible(i) && matches(i))
            .filter(|&i| !self.children(i).into_iter().any(matches))
            .collect()
    }
}

fn collect(node: &NodeRef, parent: Option<usize>, els: &mut Vec<El>, mute: bool) -> String {
    let mut text = String::new();
    for child in node.children_it(false) {
        if child.is_text() {
            if !mute {
                text.push_str(&child.text());
            }
        } else if child.is_element() {
            let tag = child.node_name().map(|name| name.to_lowercase()).unwrap_or_default();
            let attrs: Vec<(String, String)> =
                child.attrs().iter().map(|a| (a.name.local.to_string(), a.value.to_string())).collect();
            let index = els.len();
            let quiet = mute || NO_TEXT.contains(&tag.as_str());
            let hidden = NO_TEXT.contains(&tag.as_str())
                || attrs.iter().any(|(key, value)| key == HIDDEN || (key == "type" && value.eq_ignore_ascii_case("hidden")));
            els.push(El { tag: tag.clone(), attrs, text: String::new(), parent, end: index, hidden });
            let inner = collect(&child, Some(index), els, quiet);
            els[index].end = els.len() - 1;
            els[index].text = collapse(&inner);
            if BLOCKS.contains(&tag.as_str()) {
                text.push(' ');
                text.push_str(&inner);
                text.push(' ');
            } else {
                text.push_str(&inner);
            }
        }
    }
    text
}

/// The text with every run of spaces (no-break ones too) made one space.
pub fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Words compared as a person reads them: lowercase, without accents,
/// spaces collapsed.
pub fn norm(text: &str) -> String {
    let lowered: String = text
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' | 'Á' | 'À' | 'Ä' | 'Â' => 'a',
            'é' | 'è' | 'ë' | 'ê' | 'É' | 'È' | 'Ë' | 'Ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' | 'Í' | 'Ì' | 'Ï' | 'Î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'Ó' | 'Ò' | 'Ö' | 'Ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' | 'Ú' | 'Ù' | 'Ü' | 'Û' => 'u',
            'ñ' | 'Ñ' => 'n',
            other => other,
        })
        .flat_map(char::to_lowercase)
        .collect();
    collapse(&lowered)
}

/// A label normalized, without the colon or asterisk around it.
pub fn label_norm(text: &str) -> String {
    norm(text).trim_matches(|c: char| c == ':' || c == '*' || c.is_whitespace()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_is_its_elements_in_order_with_their_text() {
        let page = Page::parse(
            "https://portal.example/x",
            r#"<html><head><title>T</title><script>var a = 1;</script></head>
            <body><div id="a">Uno <b>dos</b></div><table><tr><td>146 /&nbsp;148</td><td>Tres</td></tr></table>
            <a data-micelya="1-0" title="Siguiente"><i class="fa"></i></a>
            <span data-micelya-hidden="1">oculto</span></body></html>"#,
        );
        let div = page.els.iter().position(|e| e.attr("id") == Some("a")).unwrap();
        assert_eq!(page.el(div).text, "Uno dos");
        let td = page.els.iter().position(|e| e.tag == "td").unwrap();
        assert_eq!(page.el(td).text, "146 / 148");
        let table = page.closest(td, &["table"]).unwrap();
        assert!(page.contains(table, td));
        assert_eq!(page.el(table).text, "146 / 148 Tres");
        let icon = page.els.iter().position(|e| e.tag == "i").unwrap();
        let link = page.clickable(icon).unwrap();
        assert_eq!(page.el(link).title(), "Siguiente");
        let span = page.els.iter().position(|e| e.tag == "span").unwrap();
        assert!(!page.visible(span));
        assert!(page.visible(td));
        // The script's text is not read.
        assert!(!page.els.iter().any(|e| e.tag == "body" && e.text.contains("var a")));
    }

    #[test]
    fn labels_are_found_by_their_words_without_accents_or_colon() {
        let page = Page::parse("", "<div><label>Jurisdicción:</label><div><span>Año</span></div></div>");
        let found = page.labels(&["jurisdiccion"], 0..page.len());
        assert_eq!(found.len(), 1);
        assert_eq!(page.el(found[0]).tag, "label");
        // The deepest one: the span, not the div around it.
        let year = page.labels(&["ano"], 0..page.len());
        assert_eq!(year.len(), 1);
        assert_eq!(page.el(year[0]).tag, "span");
    }
}

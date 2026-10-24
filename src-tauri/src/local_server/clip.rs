//! The web clip (`recorte_guardar`, P-227, D-450, D-451): the «Micelya
//! Clipper» browser extension (`extension/`) sends the page it clipped;
//! it is saved as a markdown file in `<carpeta de trabajo>\Recortes web\`,
//! with the page's address and the date at the top, and the base is asked
//! for a sweep: it inventories it like any document, and Atención says
//! there are new clips. It is the only change of the contract without a
//! card: the click in the extension is the person's decision.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};
use serde::Deserialize;

/// What the extension sends.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Clip {
    pub title: String,
    pub url: String,
    pub content: String,
}

/// Saves a clip in `folder` as `<aaaa-mm-dd> <título>.md`, with the page's
/// address and the date at the top. Never overwrites: a name already there
/// gets ` (2)`, ` (3)`…
pub fn save_clip(folder: &Path, clip: &Clip, now: DateTime<Local>) -> Result<PathBuf, String> {
    let content = clip.content.trim();
    if content.is_empty() {
        return Err("El recorte llegó vacío".to_string());
    }
    std::fs::create_dir_all(folder).map_err(|e| format!("No se pudo crear {}: {e}", folder.display()))?;
    let title = one_line(&clip.title);
    let title = if title.is_empty() { "Recorte sin título".to_string() } else { title };
    let stem = format!("{} {}", now.format("%Y-%m-%d"), file_title(&title));
    let mut path = folder.join(format!("{stem}.md"));
    let mut n = 2;
    while path.exists() {
        path = folder.join(format!("{stem} ({n}).md"));
        n += 1;
    }
    let url = one_line(&clip.url);
    let markdown = format!(
        "Dirección: {}\nFecha del recorte: {}\n\n# {}\n\n{}\n",
        if url.is_empty() { "(sin dirección)" } else { url.as_str() },
        now.format("%Y-%m-%d %H:%M"),
        title,
        content
    );
    std::fs::write(&path, markdown).map_err(|e| format!("No se pudo guardar el recorte: {e}"))?;
    Ok(path)
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The title as a file name: without the signs Windows does not take, up
/// to 80 letters.
fn file_title(title: &str) -> String {
    let clean: String = title
        .chars()
        .map(|c| if c.is_control() || r#"<>:"/\|?*"#.contains(c) { ' ' } else { c })
        .collect();
    let clean = one_line(&clean);
    let short: String = clean.chars().take(80).collect();
    let short = short.trim_end_matches(['.', ' ']).to_string();
    if short.is_empty() {
        "Recorte".to_string()
    } else {
        short
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::testing::TempDir;
    use chrono::TimeZone;

    fn moment() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 3, 4, 9, 30, 0).unwrap()
    }

    #[test]
    fn a_clip_is_a_markdown_file_with_the_address_and_the_date_at_the_top() {
        let dir = TempDir::new();
        let folder = dir.path().join("Recortes web");
        let clip = Clip {
            title: "Una página: de prueba / inventada?".to_string(),
            url: "https://ejemplo.invalid/nota".to_string(),
            content: "Texto de la página inventada.".to_string(),
        };
        let path = save_clip(&folder, &clip, moment()).unwrap();
        assert_eq!(path.file_name().unwrap().to_string_lossy(), "2026-03-04 Una página de prueba inventada.md");
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("Dirección: https://ejemplo.invalid/nota\nFecha del recorte: 2026-03-04 09:30\n\n# Una página: de prueba / inventada?\n\n"), "{text}");
        assert!(text.contains("Texto de la página inventada."));
        // The same page again is another file, never the same one written over.
        let again = save_clip(&folder, &clip, moment()).unwrap();
        assert_eq!(again.file_name().unwrap().to_string_lossy(), "2026-03-04 Una página de prueba inventada (2).md");
        assert!(save_clip(&folder, &Clip { content: "  ".to_string(), ..clip }, moment()).is_err());
    }
}

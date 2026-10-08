//! fs commands seen through the source volume: a mounted origin must
//! read exactly like a folder copied into `raw/sources`, and the app
//! must never write next to the originals.

use std::path::PathBuf;

use super::*;
use crate::source_volume::SourceVolume;

struct Fixture {
    base: PathBuf,
    project: PathBuf,
    origin: PathBuf,
}

impl Fixture {
    /// A project with `origin` mounted as `raw/sources/Docs`.
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!("llmwiki-fs-volume-{}", uuid::Uuid::new_v4()));
        let project = base.join("project");
        let origin = base.join("origin");
        fs::create_dir_all(project.join("raw/sources")).unwrap();
        fs::create_dir_all(origin.join("sub")).unwrap();
        fs::write(origin.join("sub").join("nota.md"), "contenido original").unwrap();
        fs::write(origin.join("plan.org"), "* Titulo\nTexto").unwrap();
        SourceVolume::open(&project)
            .unwrap()
            .add_local_mount("Docs", &origin)
            .unwrap();
        Self {
            base,
            project,
            origin,
        }
    }

    fn logical(&self, rel: &str) -> String {
        format!(
            "{}/raw/sources/{rel}",
            self.project.to_string_lossy().replace('\\', "/")
        )
    }

    fn origin_file_count(&self) -> usize {
        walkdir::WalkDir::new(&self.origin)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .count()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mounted_files_read_through_their_logical_path() {
    let fx = Fixture::new();
    let nota = fx.logical("Docs/sub/nota.md");

    assert_eq!(read_file(nota.clone(), None).await.unwrap(), "contenido original");
    assert!(file_exists(nota.clone()).await.unwrap());
    assert_eq!(get_file_size(nota.clone()).await.unwrap(), 18);
    assert_eq!(
        get_file_md5(nota.clone()).await.unwrap(),
        get_file_md5(fx.origin.join("sub/nota.md").to_string_lossy().to_string())
            .await
            .unwrap()
    );
    assert!(get_file_modified_time(nota.clone()).await.unwrap() > 0);
    assert!(!read_file_as_base64(nota).await.unwrap().base64.is_empty());

    assert!(!file_exists(fx.logical("Docs/sub/no-existe.md")).await.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn copying_a_mounted_file_reads_from_the_origin() {
    let fx = Fixture::new();
    let dest = fx.project.join("raw/sources/copia.md");

    copy_file(
        fx.logical("Docs/sub/nota.md"),
        dest.to_string_lossy().to_string(),
    )
    .await
    .unwrap();

    assert_eq!(fs::read_to_string(dest).unwrap(), "contenido original");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn extracted_text_cache_is_written_inside_the_project() {
    let fx = Fixture::new();
    let plan = fx.logical("Docs/plan.org");
    let files_before = fx.origin_file_count();

    let extracted = preprocess_source(plan.clone(), None, |_, _| {}).await.unwrap();

    assert!(extracted.contains("# Titulo"));
    let cache = fx.project.join("raw/sources/Docs/.cache/plan.org.txt");
    assert_eq!(fs::read_to_string(&cache).unwrap(), extracted);
    assert_eq!(fx.origin_file_count(), files_before);
    assert!(!fx.origin.join(".cache").exists());

    // The cache is served for the logical path while it is fresh…
    fs::write(&cache, "desde cache").unwrap();
    assert_eq!(read_file(plan.clone(), None).await.unwrap(), "desde cache");

    // …and dropped once the original changes.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    fs::write(fx.origin.join("plan.org"), "* Nuevo\nTexto").unwrap();
    assert!(read_file(plan, None).await.unwrap().contains("# Nuevo"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn files_stored_in_the_project_are_unaffected_by_mounts() {
    let fx = Fixture::new();
    let copied = fx.project.join("raw/sources/Copiados");
    fs::create_dir_all(&copied).unwrap();
    fs::write(copied.join("a.md"), "copiado").unwrap();

    assert_eq!(
        read_file(fx.logical("Copiados/a.md"), None).await.unwrap(),
        "copiado"
    );
}

fn flatten(nodes: &[FileNode], out: &mut Vec<String>) {
    for node in nodes {
        out.push(node.path.clone());
        if let Some(children) = &node.children {
            flatten(children, out);
        }
    }
}

async fn listed(path: String) -> Vec<String> {
    let mut out = Vec::new();
    flatten(&list_directory(path, Some(true), None).await.unwrap(), &mut out);
    out
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn listing_sources_shows_mounts_under_their_logical_paths() {
    let fx = Fixture::new();
    let copied = fx.project.join("raw/sources/Copiados");
    fs::create_dir_all(&copied).unwrap();
    fs::write(copied.join("a.md"), "copiado").unwrap();

    assert_eq!(
        listed(fx.logical("")).await,
        vec![
            fx.logical("Copiados"),
            fx.logical("Copiados/a.md"),
            fx.logical("Docs"),
            fx.logical("Docs/sub"),
            fx.logical("Docs/sub/nota.md"),
            fx.logical("Docs/plan.org"),
        ]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn listing_works_from_above_and_from_inside_a_mount() {
    let fx = Fixture::new();
    let project = fx.project.to_string_lossy().replace('\\', "/");

    assert!(listed(project).await.contains(&fx.logical("Docs/sub/nota.md")));
    assert_eq!(
        listed(fx.logical("Docs/sub")).await,
        vec![fx.logical("Docs/sub/nota.md")]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_mount_replaces_the_project_folder_that_holds_its_cache() {
    let fx = Fixture::new();
    preprocess_source(fx.logical("Docs/plan.org"), None, |_, _| {}).await.unwrap();

    let paths = listed(fx.logical("")).await;

    assert_eq!(paths.iter().filter(|p| **p == fx.logical("Docs")).count(), 1);
    assert!(paths.contains(&fx.logical("Docs/plan.org")));
    assert!(!paths.iter().any(|p| p.contains("/.cache")));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unreachable_origin_still_lists_as_an_empty_folder() {
    let fx = Fixture::new();
    fs::remove_dir_all(&fx.origin).unwrap();

    assert_eq!(listed(fx.logical("")).await, vec![fx.logical("Docs")]);
}

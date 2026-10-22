//! Text recognition through Micelya Core (D-463, D-468), which reads with
//! Amazon Textract in Oregon, United States (the owner takes it, D-482).
//!
//! By pages, never the whole document (D-483): a scan of many pages often
//! puts together different things, and Core's limit of 100 pages fails at
//! the end and is paid anyway. Each page goes alone, as a JPEG in gray of at
//! most 2000 pixels on its long side, made here from the image the base
//! renders of the page (or from the photo): it carries nothing of the file
//! (no metadata; its name is `pagina-N.jpg`), and the same page always gives
//! the same bytes. For each page, the whole walk of Core, every wait with a
//! limit: ask (with the hash of the image), upload it before its permit of
//! 15 minutes ends, tell it was uploaded, look every 5 seconds until it ends
//! (Core does not tell), read the result; and always, even when something
//! failed, delete the reading in Core.
//!
//! Never paid twice: Core gives back a reading of the same image it still
//! has, in progress or done, instead of reading it again (so a page whose
//! reading could not be deleted is asked again, given back without cost,
//! and deleted then); and when asking got no answer, the list is looked at
//! before asking again, so a reading registered anyway is not left behind.
//!
//! What comes back: the lines of the page, each word with its confidence. A
//! page without lines is one Textract could not read (Core does not say so
//! otherwise): it is kept as «[ilegible]», as Codex marks what it cannot
//! read, so the screens offer it to another engine. The words read with a
//! confidence under 0.90 are kept beside the text as doubtful
//! (`recognized/<hash>/page-N.json`); the text is not changed, so the search
//! and the reading see it as it was read.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use chrono::{DateTime, Utc};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageReader};
use sha2::{Digest, Sha256};

use crate::micelya_core::client::{CoreClient, CoreError, ReadingRequest, ReadingResult, Requested, Timing};
use crate::micelya_core::{saved_connection, CoreConnection};

use super::file_hash::hex;
use super::recognition::{DoubtfulWord, PageReading, TextRecognizer, RENDER_LONG_SIDE_PX, UNREADABLE};

/// The model Core reads with today (the `provider` and `modelVersion` of
/// its results).
pub const DEFAULT_MODEL: &str = "textract-1.0";
/// What the base is told when there is no connection to read with; its
/// words make it wait and try again later (`stops_the_run`).
pub const NO_CONNECTION: &str = "There is no connection with Micelya Core: load it in Ajustes → Conexiones";
/// A word read with less confidence than this is doubtful (provisional,
/// design 2.3).
const DOUBTFUL_BELOW: f64 = 0.90;
const JPEG_QUALITY: u8 = 90;
/// The largest image Core takes (JPEG and PNG).
const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;
/// How many of the newest readings are looked at before asking again.
const LIST_LOOK: u32 = 50;
/// How long before the walk began a reading of the list is still taken
/// as asked by it: the clocks of Core and of this computer may differ.
const CLOCK_MARGIN_SECONDS: i64 = 120;

pub struct CoreRecognizer {
    model: String,
    /// The connection, when it is given (the tests); otherwise it is read
    /// from the keychain when the first page is read.
    connection: Option<CoreConnection>,
    timing: Timing,
    client: Mutex<Option<Arc<CoreClient>>>,
    /// Every reading asked for in Core, in order; each one is deleted at
    /// the end of its page.
    asked: Mutex<Vec<String>>,
}

impl CoreRecognizer {
    /// Reads with the connection of the keychain.
    pub fn new(model: &str) -> Self {
        Self::make(model, None, Timing::default())
    }

    #[cfg(test)]
    pub(crate) fn with_connection(model: &str, connection: CoreConnection, timing: Timing) -> Self {
        Self::make(model, Some(connection), timing)
    }

    fn make(model: &str, connection: Option<CoreConnection>, timing: Timing) -> Self {
        let model = model.trim();
        Self {
            model: if model.is_empty() { DEFAULT_MODEL } else { model }.to_string(),
            connection,
            timing,
            client: Mutex::new(None),
            asked: Mutex::new(Vec::new()),
        }
    }

    /// The readings asked for in Core so far.
    #[cfg(test)]
    pub(crate) fn asked(&self) -> Vec<String> {
        self.asked.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    fn client(&self) -> Result<Arc<CoreClient>, String> {
        let mut kept = self.client.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(client) = kept.as_ref() {
            return Ok(client.clone());
        }
        let connection = match &self.connection {
            Some(connection) => connection.clone(),
            None => saved_connection()
                .map_err(|e| format!("{NO_CONNECTION} ({e})"))?
                .filter(|c| !c.api.is_empty() && !c.issuer.is_empty() && !c.client_secret.is_empty())
                .ok_or_else(|| NO_CONNECTION.to_string())?,
        };
        let client = Arc::new(CoreClient::new(connection, self.timing.clone()).map_err(|e| e.message())?);
        *kept = Some(client.clone());
        Ok(client)
    }
}

impl TextRecognizer for CoreRecognizer {
    fn engine_id(&self) -> String {
        format!("core:{}", self.model)
    }

    fn recognize(&self, image: &Path) -> Result<String, String> {
        self.recognize_page(image, 1).map(|reading| reading.text)
    }

    fn recognize_page(&self, image: &Path, page: u32) -> Result<PageReading, String> {
        let jpeg = page_jpeg(image)?;
        let client = self.client()?;
        let remember = |id: &str| {
            let mut asked = self.asked.lock().unwrap_or_else(|p| p.into_inner());
            if !asked.iter().any(|known| known == id) {
                asked.push(id.to_string());
            }
        };
        let result = read_page(&client, &format!("pagina-{page}.jpg"), &jpeg, &remember)?;
        let reading = page_reading(&result);
        if let Some(read_by) = reading.read_by.as_deref().filter(|by| *by != self.model) {
            // Kept with the name of the engine chosen, so it is found again
            // and never paid twice; the page keeps who read it.
            eprintln!("[core] Micelya Core read with {read_by}, not {}: the text is kept as {}", self.model, self.engine_id());
        }
        Ok(reading)
    }
}

/// The image Core is sent of a page: a JPEG in gray of at most 2000 pixels
/// on its long side (the size the base renders pages at), turned as the
/// photo says, with nothing of the file it came from.
pub(crate) fn page_jpeg(image: &Path) -> Result<Vec<u8>, String> {
    let failed = |e: &dyn std::fmt::Display| format!("The page could not be converted to an image for Micelya Core: {e}");
    let mut decoder = ImageReader::open(image)
        .and_then(|reader| reader.with_guessed_format())
        .map_err(|e| failed(&e))?
        .into_decoder()
        .map_err(|e| failed(&e))?;
    let orientation = decoder.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut decoded = DynamicImage::from_decoder(decoder).map_err(|e| failed(&e))?;
    decoded.apply_orientation(orientation);
    let long_side = RENDER_LONG_SIDE_PX as u32;
    if decoded.width().max(decoded.height()) > long_side {
        decoded = decoded.resize(long_side, long_side, FilterType::Triangle);
    }
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, JPEG_QUALITY)
        .encode_image(&decoded.to_luma8())
        .map_err(|e| failed(&e))?;
    if jpeg.len() > MAX_IMAGE_BYTES {
        return Err(format!("The page is too large for Micelya Core: {} bytes", jpeg.len()));
    }
    Ok(jpeg)
}

/// The whole walk of one page in Core. Every reading it asked for is
/// deleted at the end, whatever happened; `asked` is told of each one.
pub(crate) fn read_page(client: &CoreClient, name: &str, bytes: &[u8], asked: &dyn Fn(&str)) -> Result<ReadingResult, String> {
    let sha256 = hex(&Sha256::digest(bytes));
    let mut readings = Vec::new();
    let outcome = walk(client, name, bytes, &sha256, &mut readings);
    let mut not_deleted: Vec<CoreError> = Vec::new();
    for id in &readings {
        asked(id);
        if let Err(error) = client.delete_reading(id) {
            not_deleted.push(error);
        }
    }
    let result = outcome?;
    match not_deleted.iter().find(|error| error.is_transient()) {
        // Not kept: asked again later, Core gives this same reading back
        // without reading it again, and it is deleted then.
        Some(error) => Err(format!("The page was read but its reading could not be deleted in Micelya Core: {}", error.message())),
        None => {
            for error in &not_deleted {
                eprintln!("[core] a reading could not be deleted in Micelya Core: {}", error.message());
            }
            Ok(result)
        }
    }
}

fn remember(readings: &mut Vec<String>, id: &str) {
    if !readings.iter().any(|known| known == id) {
        readings.push(id.to_string());
    }
}

fn walk(client: &CoreClient, name: &str, bytes: &[u8], sha256: &str, readings: &mut Vec<String>) -> Result<ReadingResult, String> {
    let requested = ask(client, name, bytes, sha256, readings).map_err(|e| e.message())?;
    let id = requested.request.id.clone();
    remember(readings, &id);
    let mut request = requested.request;
    if request.status == "in_progress" {
        // Core gives back without where to upload a reading of this same
        // page that is past its upload: it is waited for.
        if let Some(upload) = &requested.upload {
            client.upload(upload, bytes).map_err(|e| e.message())?;
            request = client.confirm_upload(&id).map_err(|e| e.message())?;
        }
    }
    let timing = client.timing();
    let started = Instant::now();
    while request.status == "in_progress" {
        if started.elapsed() >= timing.reading_cap {
            return Err(format!(
                "Micelya Core did not answer within {} seconds: the page was still being read",
                timing.reading_cap.as_secs()
            ));
        }
        std::thread::sleep(timing.poll_every);
        request = client.reading(&id).map_err(|e| e.message())?;
    }
    let failure = request.failure.clone().unwrap_or_default();
    match request.status.as_str() {
        "succeeded" => client.result(&id).map_err(|e| e.message()),
        "failed" => Err(format!("Micelya Core could not read the page: {failure}")),
        "expired" => Err(format!(
            "Micelya Core let the reading expire: the page did not arrive in time (no connection while uploading?): {failure}"
        )),
        "cancelled" => Err(format!("Micelya Core cancelled the reading of the page: {failure}")),
        other => Err(format!("Micelya Core answered something unexpected (API): a reading '{other}'")),
    }
}

/// Asks for the reading of the page. When asking got no answer, the request
/// may have arrived anyway: before asking again, the list is looked at, and
/// a reading of this page asked since this walk began, still waiting for
/// its file, joins the ones deleted at the end (asking again with the same
/// hash makes Core give it back, not a second one).
fn ask(client: &CoreClient, name: &str, bytes: &[u8], sha256: &str, readings: &mut Vec<String>) -> Result<Requested, CoreError> {
    let began = Utc::now();
    let mut waits = client.timing().retry_waits.clone().into_iter();
    loop {
        match client.request_reading(name, bytes.len(), "image/jpeg", sha256) {
            Err(error) if error.is_transient() => {
                let Some(wait) = waits.next() else {
                    return Err(error);
                };
                std::thread::sleep(wait);
                if let Ok(list) = client.list_readings(LIST_LOOK, None) {
                    for item in list.items.iter().filter(|item| asked_by_this_walk(item, name, began)) {
                        remember(readings, &item.id);
                    }
                }
            }
            other => return other,
        }
    }
}

fn asked_by_this_walk(item: &ReadingRequest, name: &str, began: DateTime<Utc>) -> bool {
    item.filename == name
        && item.status == "in_progress"
        && item.stage.as_deref() == Some("awaiting_input")
        && item
            .requested_at
            .as_deref()
            .and_then(|at| DateTime::parse_from_rfc3339(at).ok())
            .is_some_and(|at| at >= began - chrono::Duration::seconds(CLOCK_MARGIN_SECONDS))
}

/// A confidence from 0 to 1 (Textract's own goes to 100).
fn unit(confidence: f64) -> f64 {
    if confidence > 1.0 {
        confidence / 100.0
    } else {
        confidence
    }
}

/// The text of a page from its lines, with the words it doubts.
fn page_reading(result: &ReadingResult) -> PageReading {
    let read_by = match (&result.provider, &result.model_version) {
        (Some(provider), Some(version)) => Some(format!("{provider}-{version}")),
        (Some(provider), None) => Some(provider.clone()),
        _ => None,
    };
    let lines: Vec<_> = result.lines.iter().filter(|line| !line.text.trim().is_empty()).collect();
    if lines.is_empty() {
        // No lines: Textract could not read it (Core does not say so otherwise).
        let text = if result.text.trim().is_empty() { UNREADABLE.to_string() } else { result.text.trim().to_string() };
        return PageReading { text, doubtful: Vec::new(), read_by };
    }
    let text = lines.iter().map(|line| line.text.trim()).collect::<Vec<_>>().join("\n");
    let doubtful = lines
        .iter()
        .enumerate()
        .flat_map(|(index, line)| {
            line.words
                .iter()
                .filter(|word| !word.text.trim().is_empty() && unit(word.confidence) < DOUBTFUL_BELOW)
                .map(move |word| DoubtfulWord {
                    word: word.text.trim().to_string(),
                    line: index + 1,
                    percent: (unit(word.confidence) * 100.0).floor().clamp(0.0, 100.0) as u8,
                })
        })
        .collect();
    PageReading { text, doubtful, read_by }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use super::*;
    use crate::docbase::similar::classify::stops_the_run;
    use crate::micelya_core::fake::{FakeCore, Outcome};

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/source_volume/fixtures").join(name)
    }

    fn recognizer(core: &FakeCore) -> CoreRecognizer {
        CoreRecognizer::with_connection(DEFAULT_MODEL, core.connection(), FakeCore::timing())
    }

    fn scratch() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("micelya-core-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The method and path (without the query) of each request.
    fn walk_of(core: &FakeCore) -> Vec<String> {
        core.seen()
            .iter()
            .map(|seen| format!("{} {}", seen.method, seen.path.split('?').next().unwrap_or("")))
            .collect()
    }

    // Every text below is made up (R-24).
    const READ: fn() -> Outcome = || {
        Outcome::Read(vec![
            vec![("CONTRATO", 0.99), ("DE", 0.98), ("PRUEBA", 0.97)],
            vec![("Importe", 0.95), ("$", 0.93), ("1.284.650,75", 0.62)],
        ])
    };

    #[test]
    fn a_page_goes_through_the_whole_walk_and_its_reading_is_deleted() {
        let core = FakeCore::start();
        core.with(|state| state.outcome = READ());
        let recognizer = recognizer(&core);
        assert_eq!(recognizer.engine_id(), "core:textract-1.0");
        let reading = recognizer.recognize_page(&fixture("pagina-prueba.png"), 3).unwrap();
        assert_eq!(reading.text, "CONTRATO DE PRUEBA\nImporte $ 1.284.650,75");
        assert_eq!(reading.doubtful, vec![DoubtfulWord { word: "1.284.650,75".into(), line: 2, percent: 62 }]);
        assert_eq!(reading.read_by.as_deref(), Some("textract-1.0"));

        let walk = walk_of(&core);
        let expected = [
            "POST /realms/prueba/protocol/openid-connect/token",
            "POST /v1/tenant/document-readings",
            "PUT /subidas/lectura-1",
            "POST /v1/tenant/document-readings/lectura-1/uploaded",
            "GET /v1/tenant/document-readings/lectura-1",
        ];
        assert_eq!(&walk[..5], &expected, "{walk:?}");
        assert_eq!(&walk[walk.len() - 2..], ["GET /v1/tenant/document-readings/lectura-1/result", "DELETE /v1/tenant/document-readings/lectura-1"]);

        // Asked with a generic name, the size and the hash of what was uploaded: a JPEG.
        let seen = core.seen();
        let asked: serde_json::Value = serde_json::from_slice(&seen[1].body).unwrap();
        let uploaded = core.with(|state| state.readings[0].uploaded.clone().unwrap());
        assert_eq!(asked["filename"], "pagina-3.jpg");
        assert_eq!(asked["contentType"], "image/jpeg");
        assert_eq!(asked["bytes"], uploaded.len());
        assert_eq!(asked["contentSha256"], hex(&Sha256::digest(&uploaded)));
        assert_eq!(&uploaded[..2], &[0xFF, 0xD8]);
        // The upload carries the headers Core gave, and never the permit.
        assert_eq!(seen[2].header("x-amz-meta-prueba").as_deref(), Some("1"));
        assert_eq!(seen[2].header("content-type").as_deref(), Some("image/jpeg"));
        assert!(seen[2].header("authorization").is_none());
        assert!(core.with(|state| state.kept().is_empty()));
        assert_eq!(recognizer.asked(), ["lectura-1"]);
    }

    #[test]
    fn the_name_of_the_file_never_goes_to_core() {
        let core = FakeCore::start();
        let dir = scratch();
        let image = dir.join("Contrato de Ejemplo firmado por Persona Inventada.png");
        std::fs::copy(fixture("pagina-prueba.png"), &image).unwrap();
        recognizer(&core).recognize_page(&image, 1).unwrap();
        for seen in core.seen() {
            let all = seen.everything();
            for part in ["Contrato de Ejemplo", "firmado", "Persona Inventada", ".png", "micelya-core-test"] {
                assert!(!all.contains(part), "{part} went to Core in {}", seen.path);
            }
        }
        let uploaded = core.with(|state| state.readings[0].uploaded.clone().unwrap());
        assert!(!uploaded.windows(7).any(|w| w == b"firmado"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_page_without_lines_is_kept_as_unread() {
        let core = FakeCore::start();
        core.with(|state| state.outcome = Outcome::Read(Vec::new()));
        let reading = recognizer(&core).recognize_page(&fixture("pagina-prueba.png"), 1).unwrap();
        assert_eq!(reading.text, UNREADABLE);
        assert!(reading.doubtful.is_empty());
        assert!(core.with(|state| state.kept().is_empty()));
    }

    #[test]
    fn a_failed_reading_says_why_in_the_words_of_core_and_is_deleted() {
        let core = FakeCore::start();
        core.with(|state| state.outcome = Outcome::Fails("El proveedor no pudo leer el documento."));
        let error = recognizer(&core).recognize_page(&fixture("pagina-prueba.png"), 1).unwrap_err();
        assert!(error.contains("could not read the page: El proveedor no pudo leer el documento."), "{error}");
        // A failure of the page, not of the provider: the document is marked.
        assert!(!stops_the_run(&error));
        assert!(core.with(|state| state.kept().is_empty()));
    }

    #[test]
    fn an_expired_reading_waits_for_later_and_is_deleted() {
        let core = FakeCore::start();
        core.with(|state| state.outcome = Outcome::Expires);
        let error = recognizer(&core).recognize_page(&fixture("pagina-prueba.png"), 1).unwrap_err();
        assert!(error.contains("expire"), "{error}");
        assert!(stops_the_run(&error));
        assert!(core.with(|state| state.kept().is_empty()));
    }

    #[test]
    fn a_reading_that_does_not_end_is_left_at_its_limit_and_deleted() {
        let core = FakeCore::start();
        core.with(|state| state.outcome = Outcome::NeverEnds);
        let timing = Timing { reading_cap: Duration::from_millis(60), ..FakeCore::timing() };
        let recognizer = CoreRecognizer::with_connection(DEFAULT_MODEL, core.connection(), timing);
        let error = recognizer.recognize_page(&fixture("pagina-prueba.png"), 1).unwrap_err();
        assert!(error.contains("did not answer within"), "{error}");
        assert!(stops_the_run(&error));
        assert!(core.with(|state| state.kept().is_empty()));
        assert_eq!(walk_of(&core).last().unwrap(), "DELETE /v1/tenant/document-readings/lectura-1");
    }

    #[test]
    fn asking_again_looks_at_the_list_first_and_never_pays_twice() {
        let core = FakeCore::start();
        // The first request is registered but its answer is lost.
        core.with(|state| state.request_failures = vec![503]);
        let recognizer = recognizer(&core);
        recognizer.recognize_page(&fixture("pagina-prueba.png"), 2).unwrap();
        let walk = walk_of(&core);
        let asks: Vec<usize> = walk.iter().enumerate().filter(|(_, w)| *w == "POST /v1/tenant/document-readings").map(|(i, _)| i).collect();
        assert_eq!(asks.len(), 2, "{walk:?}");
        assert!(walk[asks[0]..asks[1]].iter().any(|w| w == "GET /v1/tenant/document-readings"), "{walk:?}");
        // One reading only, read once, and deleted.
        assert_eq!(core.with(|state| state.readings.len()), 1);
        assert_eq!(walk.iter().filter(|w| w.starts_with("PUT ")).count(), 1);
        assert!(core.with(|state| state.kept().is_empty()));
        assert_eq!(recognizer.asked(), ["lectura-1"]);
    }

    #[test]
    fn a_page_whose_reading_could_not_be_deleted_is_given_back_without_reading_it_again() {
        let core = FakeCore::start();
        core.with(|state| {
            state.outcome = READ();
            state.delete_failures = vec![503, 503, 503];
        });
        let recognizer = recognizer(&core);
        let error = recognizer.recognize_page(&fixture("pagina-prueba.png"), 1).unwrap_err();
        assert!(error.contains("could not be deleted") && stops_the_run(&error), "{error}");
        assert_eq!(core.with(|state| state.kept().len()), 1);

        // Asked again: the same reading, not uploaded nor read again, and deleted.
        let reading = recognizer.recognize_page(&fixture("pagina-prueba.png"), 1).unwrap();
        assert!(reading.text.starts_with("CONTRATO DE PRUEBA"));
        assert_eq!(core.with(|state| state.readings.len()), 1);
        assert_eq!(walk_of(&core).iter().filter(|w| w.starts_with("PUT ")).count(), 1);
        assert!(core.with(|state| state.kept().is_empty()));
    }

    #[test]
    fn the_permit_is_asked_again_when_core_refuses_it_between_pages() {
        let core = FakeCore::start();
        let recognizer = recognizer(&core);
        recognizer.recognize_page(&fixture("pagina-prueba.png"), 1).unwrap();
        core.with(|state| state.refuse_permits_given_before = state.permits_given + 1);
        recognizer.recognize_page(&fixture("pagina-prueba.png"), 2).unwrap();
        assert_eq!(core.with(|state| state.permits_given), 2);
        assert!(core.with(|state| state.kept().is_empty()));
    }

    #[test]
    fn a_large_photo_goes_smaller_and_without_its_metadata() {
        let dir = scratch();
        let photo = dir.join("foto.png");
        image::RgbImage::from_pixel(3000, 1500, image::Rgb([250, 250, 250])).save(&photo).unwrap();
        let jpeg = page_jpeg(&photo).unwrap();
        let back = image::load_from_memory(&jpeg).unwrap();
        assert_eq!((back.width(), back.height()), (2000, 1000));
        assert!(page_jpeg(&dir.join("no-existe.png")).unwrap_err().contains("could not be converted"));
        assert!(stops_the_run(NO_CONNECTION));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// A PDF of these pages, each one lines of text in Helvetica (every
    /// text invented).
    fn invented_pdf(pages: &[&[&str]]) -> Vec<u8> {
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            String::new(),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_string(),
        ];
        let mut kids = Vec::new();
        for lines in pages {
            let content = lines
                .iter()
                .enumerate()
                .map(|(index, line)| format!("BT /F1 13 Tf 60 {} Td ({line}) Tj ET", 740 - 22 * index))
                .collect::<Vec<_>>()
                .join("\n");
            objects.push(format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()));
            let contents = objects.len();
            objects.push(format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents {contents} 0 R /Resources << /Font << /F1 3 0 R >> >> >>"
            ));
            kids.push(format!("{} 0 R", objects.len()));
        }
        objects[1] = format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), pages.len());
        let mut pdf = String::from("%PDF-1.4\n");
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.push_str(&format!("{} 0 obj\n{object}\nendobj\n", index + 1));
        }
        let xref = pdf.len();
        pdf.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1));
        for offset in offsets {
            pdf.push_str(&format!("{offset:010} 00000 n \n"));
        }
        pdf.push_str(&format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1));
        pdf.into_bytes()
    }

    /// Talks to the real Micelya Core with the connection of the keychain
    /// and costs cents, so it only runs on request, once:
    /// `cargo test --lib real_core -- --ignored --nocapture`.
    /// The document is invented: a letter, a receipt and a blank page.
    #[test]
    #[ignore = "uses the real Micelya Core with the credential of the keychain; costs cents"]
    fn real_core_reads_an_invented_document_page_by_page_and_deletes_every_reading() {
        let connection = saved_connection().unwrap().expect("the connection with Micelya Core in the keychain");
        let clock = Instant::now();
        let test = crate::micelya_core::test_connection(&connection, Timing::default());
        println!("probar la conexión: {test:?} ({:.1} s)", clock.elapsed().as_secs_f64());
        assert!(test.ok);

        let dir = scratch();
        let pdf = dir.join("documento-inventado.pdf");
        std::fs::write(
            &pdf,
            invented_pdf(&[
                &[
                    "CARTA DE PRUEBA",
                    "Ciudad Inventada, 3 de marzo de 2026",
                    "Estimada Persona Inventada:",
                    "Le informamos que el expediente ficticio numero 1234/2026",
                    "quedo archivado. Esta carta es inventada y no se refiere a nadie.",
                    "Saludos cordiales,",
                    "Estudio Ejemplo",
                ],
                &[
                    "RECIBO DE PRUEBA N. 0001-00004567",
                    "Recibi de Persona Inventada la suma de pesos 12.345,67",
                    "en concepto de honorarios ficticios del mes de abril.",
                    "CUIT 30-11111111-1",
                ],
                &[],
            ]),
        )
        .unwrap();
        let recognizer = CoreRecognizer::with_connection(DEFAULT_MODEL, connection.clone(), Timing::default());
        let mut texts = Vec::new();
        for page in 1..=3 {
            let image = super::super::recognition::render_page(&pdf, page, &dir).unwrap();
            let started = Instant::now();
            let reading = recognizer.recognize_page(&image, page).unwrap();
            println!(
                "página {page}: {:.1} s, {} renglones, {} palabras dudosas, leída por {:?}\n{}\n",
                started.elapsed().as_secs_f64(),
                reading.text.lines().count(),
                reading.doubtful.len(),
                reading.read_by,
                reading.text
            );
            texts.push(reading.text);
        }
        println!("en total: {:.1} s, {} lecturas", clock.elapsed().as_secs_f64(), recognizer.asked().len());
        assert!(texts[0].contains("CARTA DE PRUEBA"), "{}", texts[0]);
        assert!(texts[1].contains("RECIBO DE PRUEBA"), "{}", texts[1]);
        assert_eq!(texts[2], UNREADABLE);

        // Every reading was deleted in Core.
        let client = CoreClient::new(connection, Timing::default()).unwrap();
        for id in recognizer.asked() {
            assert_eq!(client.reading(&id).unwrap_err().status(), Some(404), "the reading {id} is still in Core");
        }
        let _ = std::fs::remove_dir_all(dir);
    }
}

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
//! reading could not be deleted for a moment is asked again, given back
//! without cost, and deleted then); and when asking got no answer, the
//! list is looked at before asking again, so a reading registered anyway is
//! not left behind.
//!
//! Only its own (P-294): a credential may be shared by several computers,
//! and the name of a page says nothing of whose it is. Every reading Core
//! gives this computer is kept in the ledger (`micelya_core::ledger`) by
//! its identifier until it is deleted, and only those are ever deleted.
//! When asking got no answer, the request may have been registered anyway:
//! asked again with the same hash, Core gives it back (it waits for its
//! file, and was asked by the same person), and that one is this
//! computer's; of the list, only requests of the ledger join the ones
//! deleted. A reading Core gives back that is not in the ledger (another
//! computer reads the same image) is read, never deleted: its computer
//! deletes it. A reading Core refuses to delete for something that is not a
//! cut (a permission) keeps its page, stays in the ledger, is shown in
//! Atención and is tried again later.
//!
//! Several pages at once (P-292): each page takes 15 to 19 seconds in Core,
//! nearly all of it waiting, so up to `PAGES_AT_ONCE` pages of a document
//! go at the same time, each with its whole walk (its own reading, deleted
//! at its end) on a thread of its own. Each page is kept in its own section
//! whatever the order they end in. After a page fails no other page
//! starts; those under way end, so what Core read is kept and every
//! reading is deleted. Two pages of a scan with the same image are read
//! once: the second waits for the first, or takes its reading if it is done.
//!
//! What comes back: the lines of the page, each word with its confidence. A
//! page without lines is one Textract could not read (Core does not say so
//! otherwise): it is kept as «[ilegible]», as Codex marks what it cannot
//! read, so the screens offer it to another engine. The words read with a
//! confidence under 0.90 are kept beside the text as doubtful
//! (`recognized/<hash>/page-N.json`); the text is not changed, so the search
//! and the reading see it as it was read.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use chrono::{DateTime, Utc};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageReader};
use sha2::{Digest, Sha256};

use crate::micelya_core::client::{CoreClient, CoreError, ReadingRequest, ReadingResult, Requested, Timing};
use crate::micelya_core::ledger::{self, Ledger};
use crate::micelya_core::{saved_connection, CoreConnection};

use super::file_hash::hex;
use super::recognition::{pages_at_once, DoubtfulWord, PageImage, PageReading, TextRecognizer, RENDER_LONG_SIDE_PX, UNREADABLE};

/// The model Core reads with today (the `provider` and `modelVersion` of
/// its results).
pub const DEFAULT_MODEL: &str = "textract-1.0";
/// How many pages of a document are read in Core at the same time (P-292).
/// Core publishes no limit per minute and does not answer 429; with 4, a
/// scan of 23 pages takes about 6 rounds of 15 to 19 seconds instead of 23.
/// To change it, change this number (1 reads one page after the other).
pub const PAGES_AT_ONCE: usize = 4;
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
    /// Every reading of this computer a page walked, in order; each one is
    /// deleted at the end of its page.
    asked: Mutex<Vec<String>>,
    /// The readings of this computer not deleted yet in Core.
    ledger: Arc<Ledger>,
    /// How many pages are read at the same time.
    at_once: usize,
    /// The readings of the pages of the scans under way, by the hash of
    /// their image: a page with the same image waits for its reading or
    /// takes it. Let go at the end of each scan.
    same_image: Mutex<HashMap<String, Arc<Mutex<Option<PageReading>>>>>,
}

impl CoreRecognizer {
    /// Reads with the connection of the keychain, and the ledger of the app.
    pub fn new(model: &str) -> Self {
        Self::make(model, None, Timing::default(), ledger::shared())
    }

    /// With a ledger of its own, only in memory.
    #[cfg(test)]
    pub(crate) fn with_connection(model: &str, connection: CoreConnection, timing: Timing) -> Self {
        Self::make(model, Some(connection), timing, Arc::new(Ledger::in_memory()))
    }

    #[cfg(test)]
    pub(crate) fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    /// The same, reading `at_once` pages at the same time.
    #[cfg(test)]
    pub(crate) fn reading_at_once(mut self, at_once: usize) -> Self {
        self.at_once = at_once.max(1);
        self
    }

    fn make(model: &str, connection: Option<CoreConnection>, timing: Timing, ledger: Arc<Ledger>) -> Self {
        let model = model.trim();
        Self {
            model: if model.is_empty() { DEFAULT_MODEL } else { model }.to_string(),
            connection,
            timing,
            client: Mutex::new(None),
            asked: Mutex::new(Vec::new()),
            ledger,
            at_once: PAGES_AT_ONCE,
            same_image: Mutex::new(HashMap::new()),
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
        let reading = self.recognize_page(image, 1).map(|reading| reading.text);
        self.let_go();
        reading
    }

    fn recognize_page(&self, image: &Path, page: u32) -> Result<PageReading, String> {
        let jpeg = page_jpeg(image)?;
        let client = self.client()?;
        // A page with the same image as another of the same scan waits for
        // its reading, or takes it when it is done: the reading stays until
        // the scan ends, so a page that starts late does not pay again.
        let sha256 = hex(&Sha256::digest(&jpeg));
        let slot = {
            let mut same_image = self.same_image.lock().unwrap_or_else(|p| p.into_inner());
            if same_image.len() > MAX_SAME_IMAGE {
                same_image.retain(|_, slot| Arc::strong_count(slot) > 1);
            }
            same_image.entry(sha256).or_default().clone()
        };
        let mut same = slot.lock().unwrap_or_else(|p| p.into_inner());
        match same.as_ref() {
            Some(reading) => Ok(reading.clone()),
            None => {
                let read = self.read_one(&client, page, &jpeg);
                if let Ok(reading) = &read {
                    *same = Some(reading.clone());
                }
                read
            }
        }
    }

    fn recognize_pages(&self, pages: &[u32], image: &PageImage<'_>, done: &mut dyn FnMut(u32, Result<PageReading, String>) -> bool) {
        pages_at_once(self.at_once, pages, &|page| image(page).and_then(|image| self.recognize_page(&image, page)), done);
        self.let_go();
    }
}

/// How many readings of an image are kept for the pages of a scan before the
/// ones nobody is waiting for are let go (pages read one by one, outside a
/// scan, would keep them otherwise).
const MAX_SAME_IMAGE: usize = 256;

impl CoreRecognizer {
    /// Lets go of the readings of an image that no page is using any more:
    /// at the end of a scan, or of one image.
    fn let_go(&self) {
        self.same_image.lock().unwrap_or_else(|p| p.into_inner()).retain(|_, slot| Arc::strong_count(slot) > 1);
    }

    /// The whole walk of one page in Core, as `pagina-N.jpg`.
    fn read_one(&self, client: &CoreClient, page: u32, jpeg: &[u8]) -> Result<PageReading, String> {
        let remember = |id: &str| {
            let mut asked = self.asked.lock().unwrap_or_else(|p| p.into_inner());
            if !asked.iter().any(|known| known == id) {
                asked.push(id.to_string());
            }
        };
        let result = read_page(client, &self.ledger, &format!("pagina-{page}.jpg"), jpeg, &remember)?;
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

/// The whole walk of one page in Core. Every reading of this computer it
/// walked is deleted at the end, whatever happened; `asked` is told of each
/// one, and `ledger` of how each ended.
pub(crate) fn read_page(client: &CoreClient, ledger: &Ledger, name: &str, bytes: &[u8], asked: &dyn Fn(&str)) -> Result<ReadingResult, String> {
    let sha256 = hex(&Sha256::digest(bytes));
    let mut readings = Vec::new();
    let outcome = walk(client, ledger, name, bytes, &sha256, &mut readings);
    let mut not_deleted: Vec<(String, CoreError)> = Vec::new();
    for id in &readings {
        asked(id);
        match client.delete_reading(id) {
            Ok(()) => ledger.deleted(id),
            Err(error) => not_deleted.push((id.clone(), error)),
        }
    }
    // The page is kept when it was read and no reading waits for a cut to end.
    let kept = outcome.is_ok() && !not_deleted.iter().any(|(_, error)| error.is_transient());
    for (id, error) in &not_deleted {
        let refused = (!error.is_transient()).then(|| error.message());
        if let Some(reason) = &refused {
            eprintln!("[core] a reading could not be deleted in Micelya Core; it shows in Atención and is tried again later: {reason}");
        }
        ledger.not_deleted(id, kept, refused);
    }
    let result = outcome?;
    match not_deleted.iter().find(|(_, error)| error.is_transient()) {
        // Not kept: asked again later, Core gives this same reading back
        // without reading it again, and it is deleted then.
        Some((_, error)) => Err(format!("The page was read but its reading could not be deleted in Micelya Core: {}", error.message())),
        None => Ok(result),
    }
}

fn remember(readings: &mut Vec<String>, id: &str) {
    if !readings.iter().any(|known| known == id) {
        readings.push(id.to_string());
    }
}

fn walk(client: &CoreClient, ledger: &Ledger, name: &str, bytes: &[u8], sha256: &str, readings: &mut Vec<String>) -> Result<ReadingResult, String> {
    let requested = ask(client, ledger, name, bytes, sha256, readings).map_err(|e| e.message())?;
    let id = requested.request.id.clone();
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

/// Asks for the reading of the page; the readings of this computer go to
/// `readings`, deleted at the end. When asking got no answer, the request
/// may have arrived anyway: before asking again, the list is looked at, and
/// a request of this computer (of the ledger) for this page since this walk
/// began, still waiting for its file, joins them; a request of the list not
/// in the ledger is never taken, whatever its name. The request registered
/// by the lost answer itself comes back when asking again with the same
/// hash (it waits for its file, and was asked by the same person): this
/// computer's, it joins them too.
fn ask(client: &CoreClient, ledger: &Ledger, name: &str, bytes: &[u8], sha256: &str, readings: &mut Vec<String>) -> Result<Requested, CoreError> {
    let began = Utc::now();
    let mut lost = false;
    let mut waits = client.timing().retry_waits.clone().into_iter();
    loop {
        match client.request_reading(name, bytes.len(), "image/jpeg", sha256) {
            Err(error) if error.is_transient() => {
                let Some(wait) = waits.next() else {
                    return Err(error);
                };
                lost = true;
                std::thread::sleep(wait);
                if let Ok(list) = client.list_readings(LIST_LOOK, None) {
                    for item in list.items.iter().filter(|item| asked_by_this_walk(item, name, began)) {
                        if !readings.contains(&item.id) && ledger.adopt(&item.id) {
                            readings.push(item.id.clone());
                        }
                    }
                }
            }
            Ok(requested) => {
                let id = requested.request.id.as_str();
                let waiting = requested.request.stage.as_deref() == Some("awaiting_input");
                let ours = !requested.found || readings.iter().any(|known| known == id) || ledger.adopt(id) || (lost && waiting);
                if ours {
                    ledger.asked(id, name);
                    remember(readings, id);
                } else {
                    eprintln!("[core] Micelya Core gave back a reading of the same page asked by someone else: it is read, not deleted");
                }
                return Ok(requested);
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
    use std::collections::BTreeMap;
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
    fn a_request_of_another_computer_is_never_deleted_when_asking_again() {
        let core = FakeCore::start();
        core.with(|state| {
            state.outcome = READ();
            // Another computer with the same credential asked for its page 2
            // a moment ago and has not uploaded it yet.
            state.of_another_computer("de-otra-computadora", "pagina-2.jpg", &"0".repeat(64), "in_progress", Some("awaiting_input"));
            // Ours is registered but its answer is lost.
            state.request_failures = vec![503];
        });
        let recognizer = recognizer(&core);
        let reading = recognizer.recognize_page(&fixture("pagina-prueba.png"), 2).unwrap();
        assert!(reading.text.starts_with("CONTRATO DE PRUEBA"));
        let walk = walk_of(&core);
        // The list was looked at before asking again; both requests were in it.
        let asks: Vec<usize> = walk.iter().enumerate().filter(|(_, w)| *w == "POST /v1/tenant/document-readings").map(|(i, _)| i).collect();
        assert!(walk[asks[0]..asks[1]].iter().any(|w| w == "GET /v1/tenant/document-readings"), "{walk:?}");
        // Ours (given back by Core when asked again) was read and deleted;
        // the other computer's was never touched.
        assert!(!walk.iter().any(|w| w.contains("de-otra-computadora")), "{walk:?}");
        let kept: Vec<String> = core.with(|state| state.kept().iter().map(|r| r.id.clone()).collect());
        assert_eq!(kept, ["de-otra-computadora"]);
        assert_eq!(recognizer.asked(), ["lectura-2"]);
        assert!(recognizer.ledger().left().is_empty());
    }

    #[test]
    fn a_reading_of_the_same_image_asked_by_another_computer_is_read_but_never_deleted() {
        let core = FakeCore::start();
        let image = fixture("pagina-prueba.png");
        let sha256 = hex(&Sha256::digest(page_jpeg(&image).unwrap()));
        core.with(|state| {
            state.outcome = READ();
            state.of_another_computer("de-otra-computadora", "pagina-1.jpg", &sha256, "succeeded", None);
        });
        let recognizer = recognizer(&core);
        let reading = recognizer.recognize_page(&image, 1).unwrap();
        assert!(reading.text.starts_with("CONTRATO DE PRUEBA"));
        // Read without paying again, and left to the computer that asked for it.
        assert_eq!(core.with(|state| state.readings.len()), 1);
        assert_eq!(core.with(|state| state.kept().len()), 1);
        assert!(!walk_of(&core).iter().any(|w| w.starts_with("DELETE ") || w.starts_with("PUT ")), "{:?}", walk_of(&core));
        assert!(recognizer.asked().is_empty() && recognizer.ledger().left().is_empty());
    }

    #[test]
    fn a_reading_core_refuses_to_delete_keeps_its_page_shows_in_attention_and_is_deleted_later() {
        let core = FakeCore::start();
        core.with(|state| {
            state.outcome = READ();
            // The credential cannot delete: not a cut.
            state.delete_failures = vec![403];
        });
        let recognizer = recognizer(&core);
        let reading = recognizer.recognize_page(&fixture("pagina-prueba.png"), 1).unwrap();
        assert!(reading.text.starts_with("CONTRATO DE PRUEBA"));
        assert_eq!(core.with(|state| state.kept().len()), 1);
        let refused = recognizer.ledger().refused();
        assert_eq!(refused.len(), 1);
        assert_eq!((refused[0].id.as_str(), refused[0].name.as_str(), refused[0].kept), ("lectura-1", "pagina-1.jpg", true));
        assert!(refused[0].refused.as_deref().unwrap().contains("403"), "{refused:?}");

        // Tried again later, it is deleted and leaves Atención.
        let client = CoreClient::new(core.connection(), FakeCore::timing()).unwrap();
        assert_eq!(ledger::delete_left(&client, recognizer.ledger()), 1);
        assert!(core.with(|state| state.kept().is_empty()));
        assert!(recognizer.ledger().left().is_empty());
    }

    #[test]
    fn the_permit_is_asked_again_when_core_refuses_it_between_pages() {
        let core = FakeCore::start();
        let recognizer = recognizer(&core);
        // Two different images: a page with the same image as an earlier one
        // of the same scan takes its reading and asks Core nothing.
        let images = images(&scratch(), 2);
        recognizer.recognize_page(&images[0], 1).unwrap();
        core.with(|state| state.refuse_permits_given_before = state.permits_given + 1);
        recognizer.recognize_page(&images[1], 2).unwrap();
        assert_eq!(core.with(|state| state.permits_given), 2);
        assert!(core.with(|state| state.kept().is_empty()));
    }

    /// `count` images of a page, each a different gray (each one is a
    /// different image for Core).
    fn images(dir: &Path, count: u8) -> Vec<PathBuf> {
        (1..=count)
            .map(|n| {
                let path = dir.join(format!("imagen-{n}.png"));
                image::GrayImage::from_pixel(60, 40, image::Luma([n * 20])).save(&path).unwrap();
                path
            })
            .collect()
    }

    #[test]
    fn several_pages_are_read_at_once_never_more_than_the_limit_each_with_its_own_text() {
        let core = FakeCore::start();
        // Each reading takes a while, so the pages overlap.
        core.with(|state| {
            state.outcome = Outcome::Named;
            state.looks_to_end = 30;
        });
        let dir = scratch();
        let images = images(&dir, 9);
        let recognizer = recognizer(&core);
        let pages: Vec<u32> = (1..=9).collect();
        let mut texts = BTreeMap::new();
        recognizer.recognize_pages(&pages, &|page| Ok(images[page as usize - 1].clone()), &mut |page, reading| {
            texts.insert(page, reading.unwrap().text);
            true
        });
        assert_eq!(texts.len(), 9);
        for (page, text) in &texts {
            assert_eq!(text, &format!("Lectura de pagina-{page}.jpg"));
        }
        assert_eq!(core.with(|state| state.open_most), PAGES_AT_ONCE);
        // One reading per page, each with its generic name, every one deleted.
        let mut names: Vec<String> = core.with(|state| state.readings.iter().map(|r| r.filename.clone()).collect());
        names.sort();
        assert_eq!(names, (1..=9).map(|n| format!("pagina-{n}.jpg")).collect::<Vec<_>>());
        assert!(core.with(|state| state.kept().is_empty()));
        assert_eq!(recognizer.asked().len(), 9);
        // One permit for all of them.
        assert_eq!(core.with(|state| state.permits_given), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn when_a_page_fails_the_pages_under_way_end_and_every_reading_is_deleted() {
        let core = FakeCore::start();
        core.with(|state| {
            state.outcome = Outcome::Named;
            state.looks_to_end = 30;
            state.fails_named = vec!["pagina-2.jpg".into()];
        });
        let dir = scratch();
        let images = images(&dir, 8);
        let recognizer = recognizer(&core);
        let pages: Vec<u32> = (1..=8).collect();
        let mut ended = BTreeMap::new();
        recognizer.recognize_pages(&pages, &|page| Ok(images[page as usize - 1].clone()), &mut |page, reading| {
            ended.insert(page, reading);
            true
        });
        // The pages under way when it failed ended and were read; no other started.
        assert_eq!(ended.keys().copied().collect::<Vec<_>>(), [1, 2, 3, 4]);
        let error = ended[&2].as_ref().unwrap_err();
        assert!(error.contains("could not read the page: El proveedor no pudo leer esta página."), "{error}");
        for page in [1, 3, 4] {
            assert_eq!(ended[&page].as_ref().unwrap().text, format!("Lectura de pagina-{page}.jpg"));
        }
        assert_eq!(core.with(|state| state.readings.len()), 4);
        assert!(core.with(|state| state.kept().is_empty()));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn pages_with_the_same_image_at_the_same_time_are_read_once() {
        let core = FakeCore::start();
        core.with(|state| {
            state.outcome = READ();
            state.looks_to_end = 20;
        });
        let image = fixture("pagina-prueba.png");
        let recognizer = recognizer(&core);
        let mut texts = BTreeMap::new();
        recognizer.recognize_pages(&[1, 2, 3], &|_| Ok(image.clone()), &mut |page, reading| {
            texts.insert(page, reading.unwrap().text);
            true
        });
        assert_eq!(texts.len(), 3);
        assert!(texts.values().all(|text| text.starts_with("CONTRATO DE PRUEBA")), "{texts:?}");
        assert_eq!(core.with(|state| state.readings.len()), 1);
        assert!(core.with(|state| state.kept().is_empty()));
    }

    /// The text between `## Page {page}` and the next page.
    fn section(text: &str, page: u32) -> &str {
        let heading = format!("## Page {page}\n");
        let start = text.find(&heading).unwrap_or_else(|| panic!("no page {page} in {text}")) + heading.len();
        let end = text[start..].find("## Page ").map_or(text.len(), |next| start + next);
        &text[start..end]
    }

    #[test]
    fn a_scan_read_several_pages_at_once_keeps_each_page_in_its_own_section() {
        let core = FakeCore::start();
        core.with(|state| {
            state.outcome = Outcome::Named;
            state.looks_to_end = 10;
        });
        let dir = scratch();
        let pdf = dir.join("escaneo.pdf");
        let lines: Vec<String> = (1..=6).map(|n| format!("Hoja {n}")).collect();
        let pages: Vec<[&str; 1]> = lines.iter().map(|line| [line.as_str()]).collect();
        let pages: Vec<&[&str]> = pages.iter().map(|page| &page[..]).collect();
        std::fs::write(&pdf, invented_pdf(&pages)).unwrap();
        let extracted = (1..=6).map(|n| format!("## Page {n}\n\n\n")).collect::<Vec<_>>().join("\n");
        let store = super::super::recognized_store::RecognizedStore::at(dir.join("leidas"));
        // Its pages are read as scans whatever their ficha says: what is
        // measured is reading several at once.
        let all: Vec<u32> = (1..=6).collect();
        let text = super::super::recognition::complete_pdf_pages(&pdf, &all, &Default::default(), &extracted, &store, &recognizer(&core), &|_, _| {})
            .unwrap();
        for page in 1..=6 {
            let own = section(&text, page);
            assert!(own.contains(&format!("Lectura de pagina-{page}.jpg")), "page {page}: {own}");
            assert_eq!(own.matches("Lectura de").count(), 1, "page {page}: {own}");
        }
        assert_eq!(core.with(|state| state.readings.len()), 6);
        assert!(core.with(|state| state.kept().is_empty()));
        let _ = std::fs::remove_dir_all(dir);
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
    /// The document is invented: eight pages of a scan (a letter, a receipt
    /// and other made-up papers, and a blank page), read through the base
    /// one page at a time and then `PAGES_AT_ONCE` at once, to tell how
    /// long each takes.
    #[test]
    #[ignore = "uses the real Micelya Core with the credential of the keychain; costs cents"]
    fn real_core_reads_an_invented_scan_one_page_at_a_time_and_several_at_once_and_deletes_every_reading() {
        let connection = saved_connection().unwrap().expect("the connection with Micelya Core in the keychain");
        let clock = Instant::now();
        let test = crate::micelya_core::test_connection(&connection, Timing::default());
        println!("probar la conexión: {test:?} ({:.1} s)", clock.elapsed().as_secs_f64());
        assert!(test.ok);

        // Few words per page, as a scan reads.
        let pages: [&[&str]; 8] = [
            &["CARTA DE PRUEBA", "Ciudad Inventada, 3 de marzo", "Persona Inventada"],
            &["RECIBO DE PRUEBA 0001", "Importe 12.345,67", "CUIT 30-11111111-1"],
            &["NOTA FICTICIA 3", "Expediente 1234/2026"],
            &["ACTA DE PRUEBA 4", "Sin valor legal"],
            &["FACTURA INVENTADA 5", "Total 987,65"],
            &["CONSTANCIA FICTICIA 6", "Estudio Ejemplo"],
            &["PODER DE PRUEBA 7", "Otorgante Inventado"],
            &[],
        ];
        let first_words = ["CARTA", "RECIBO", "NOTA", "ACTA", "FACTURA", "CONSTANCIA", "PODER"];
        let dir = scratch();
        let pdf = dir.join("escaneo-inventado.pdf");
        std::fs::write(&pdf, invented_pdf(&pages)).unwrap();
        // Read as scans whatever their ficha says (they have no image).
        let all: Vec<u32> = (1..=8).collect();
        let extracted = (1..=8).map(|n| format!("## Page {n}\n\n\n")).collect::<Vec<_>>().join("\n");

        let mut times = Vec::new();
        for at_once in [1, PAGES_AT_ONCE] {
            let recognizer = CoreRecognizer::with_connection(DEFAULT_MODEL, connection.clone(), Timing::default()).reading_at_once(at_once);
            let store = super::super::recognized_store::RecognizedStore::at(dir.join(format!("leidas-{at_once}")));
            let started = Instant::now();
            let text = super::super::recognition::complete_pdf_pages(&pdf, &all, &Default::default(), &extracted, &store, &recognizer, &|_, _| {})
                .unwrap();
            let seconds = started.elapsed().as_secs_f64();
            println!("{at_once} a la vez: {seconds:.1} s para 8 páginas, {} lecturas", recognizer.asked().len());
            for (index, word) in first_words.iter().enumerate() {
                let page = index as u32 + 1;
                let own = section(&text, page);
                println!("  página {page}: {}", own.lines().filter(|l| !l.trim().is_empty() && !l.starts_with("<!--")).collect::<Vec<_>>().join(" / "));
                assert!(own.contains(word), "page {page}: {own}");
            }
            assert!(section(&text, 8).contains(UNREADABLE), "{}", section(&text, 8));

            // Every reading was deleted in Core.
            let client = CoreClient::new(connection.clone(), Timing::default()).unwrap();
            for id in recognizer.asked() {
                assert_eq!(client.reading(&id).unwrap_err().status(), Some(404), "the reading {id} is still in Core");
            }
            times.push(seconds);
        }
        println!("antes (de a una): {:.1} s; después ({PAGES_AT_ONCE} a la vez): {:.1} s; en total {:.1} s", times[0], times[1], clock.elapsed().as_secs_f64());
        let _ = std::fs::remove_dir_all(dir);
    }
}

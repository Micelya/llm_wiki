//! The record of failures with simulated failures (no model, R-24): the
//! class of each row of the table of A2 and of the owner's four causes,
//! the attempts and their waits, the answer of another shape that turns
//! unknown, the threshold of the indicator, and the keys `failed:` that
//! come into the table.

use rusqlite::Connection;

use super::*;

/// One test per row of the table: the message, and the code, class and
/// reach `classify` gives it.
macro_rules! row {
    ($name:ident, $message:expr, $code:expr, $class:expr, $scope:expr) => {
        #[test]
        fn $name() {
            let cause = classify($message);
            assert_eq!((cause.code, cause.class, cause.scope), ($code, $class, $scope), "{}", $message);
        }
    };
}

// 1, 8 and 10. The base does not answer: busy, full or damaged.
row!(row_1_the_base_busy_is_transient, "Document base error: database is locked", "db-busy", Transient, Item);
row!(row_1_the_disk_full_is_transient, "Document base error: database or disk is full", "disk-full", Transient, Item);
row!(row_1_the_base_damaged_is_permanent, "Document base error: database disk image is malformed", "db-damaged", Permanent, Item);
row!(row_8_the_parts_with_the_base_busy, "Document base error: database is busy", "db-busy", Transient, Item);
row!(row_10_the_signature_with_the_base_busy, "Document base error: database table is locked", "db-busy", Transient, Item);
// 2. The ficha: pdfium does not open the PDF.
row!(
    row_2_an_encrypted_pdf_is_permanent,
    "Failed to open PDF 'x.pdf': PdfiumLibraryInternalError(PasswordError)",
    "pdf-encrypted",
    Permanent,
    Item
);
row!(row_2_a_broken_pdf_is_permanent, "Failed to open PDF 'x.pdf': PdfiumLibraryInternalError(FormatError)", "pdf-broken", Permanent, Item);
row!(
    row_2_a_pdf_in_use_is_transient,
    "Failed to open PDF 'x.pdf': PdfiumLibraryInternalError(FileError) (The process cannot access the file because it is being used by another process. (os error 32))",
    "file-locked",
    Transient,
    Item
);
// 3. pdfium does not open it when cutting: as the 2.
row!(
    row_3_a_pdf_locked_when_cutting_is_transient,
    "Failed to open PDF 'x.pdf': PdfiumLibraryInternalError(FileError) (The process cannot access the file because another process has locked a portion of the file. (os error 33))",
    "file-locked",
    Transient,
    Item
);
// 5 and 6. The reading by zones: another shape; no memory; anything else.
row!(
    row_5_the_reading_by_zones_of_another_shape_is_permanent,
    "The reading by zones is not of the shape expected: missing field `page`",
    "zones-shape",
    Permanent,
    Item
);
row!(row_6_the_reading_by_zones_without_memory_is_transient, "the reader ran out of memory", "out-of-memory", Transient, Item);
row!(row_6_any_other_failure_of_the_reading_by_zones_is_unknown, "the reader returned code 7", "unknown", Unknown, Item);
// 7. An error of the program in the rules.
row!(
    row_7_an_error_of_the_program_is_unknown,
    "Internal error in docbase_cut: index out of bounds",
    "program-error",
    Unknown,
    Item
);
// 11. The digital signatures: in use, transient; broken, permanent.
row!(
    row_11_signatures_of_a_pdf_in_use_are_transient,
    "Failed to open PDF 'x.pdf': PdfiumLibraryInternalError(FileError) (being used by another process)",
    "file-locked",
    Transient,
    Item
);
row!(row_11_signatures_of_a_broken_pdf_are_permanent, "Failed to open PDF 'x.pdf': PdfiumLibraryInternalError(FormatError)", "pdf-broken", Permanent, Item);
// 12. The limit, the quota, the 429: the whole engine.
row!(row_12_the_usage_limit, "Codex exited with exit code: 1: You've hit your usage limit", "rate-limit", Transient, Engine);
row!(row_12_a_429, "stream error: 429 Too Many Requests", "rate-limit", Transient, Engine);
row!(row_12_the_quota, "insufficient_quota: You exceeded your current quota", "rate-limit", Transient, Engine);
// 13. No connection, no answer in time, the engine missing.
row!(row_13_no_connection, "error sending request for url (https://example.invalid/)", "no-connection", Transient, Engine);
row!(row_13_no_answer_in_time, "Codex did not answer within 600 seconds", "timeout", Transient, Engine);
row!(row_13_the_engine_missing_is_permanent, "codex not found on PATH", "engine-missing", Permanent, Engine);
// 14. The credential rejected.
row!(row_14_the_credential_rejected, "401 Unauthorized: invalid api key", "credential", Permanent, Engine);
// 16. The image of the page that cannot be made: as the 2.
row!(
    row_16_the_image_of_an_encrypted_pdf,
    "Failed to open PDF 'x.pdf': PdfiumLibraryInternalError(PasswordError)",
    "pdf-encrypted",
    Permanent,
    Item
);
// Anything else is unknown.
row!(an_error_nobody_knows_is_unknown, "algo raro", "unknown", Unknown, Item);

// The owner's four.
row!(the_owner_question_too_large_413, "HTTP 413 Request Entity Too Large", "too-large", Permanent, Item);
row!(the_owner_question_too_large_context, "prompt is too long: maximum context length is 200000 tokens", "too-large", Permanent, Item);
row!(the_owner_credential_403, "403 Forbidden", "credential", Permanent, Engine);
row!(
    the_owner_cloud_file_not_downloaded,
    "Failed to open PDF 'x.pdf': PdfiumLibraryInternalError(FileError) (cloud file not downloaded)",
    "cloud-not-downloaded",
    Transient,
    Item
);

#[test]
fn the_causes_given_by_the_code_keep_their_class_and_reach() {
    // 15: another shape, transient of its item; 17: the folder not
    // connected, transient; 18b and the owner's: the engine that does not
    // look at images, permanent (it does not pause the engine for the
    // questions that are not about images).
    let of = |code| {
        let c = cause(code);
        (c.class, c.scope)
    };
    assert_eq!(of("bad-answer"), (Transient, Item));
    assert_eq!(of("folder-disconnected"), (Transient, Item));
    assert_eq!(of("engine-no-images"), (Permanent, Item));
    assert_eq!(of("zones-model-missing"), (Permanent, Item));
    assert_eq!(of("no-such-code"), (Unknown, Item));
}

fn base() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    super::super::schema::migrate(&mut conn).unwrap();
    conn.execute_batch(
        "INSERT INTO version (id, state, first_seen_at) VALUES (1, 'processed', 1);
         INSERT INTO file (hash, version_id, size) VALUES ('h1', 1, 10);",
    )
    .unwrap();
    conn
}

const MINUTE: i64 = 60 * 1000;

#[test]
fn a_transient_one_is_tried_again_without_end_and_the_others_never() {
    assert_eq!(
        (1..=6).map(retry_after).collect::<Vec<_>>(),
        vec![10 * MINUTE, 60 * MINUTE, 360 * MINUTE, 1440 * MINUTE, 1440 * MINUTE, 1440 * MINUTE]
    );
    let conn = base();
    let locked = Failure::new("shapes", About::File("h1".into()), "Failed to open PDF: (os error 32)");
    let mut at = 1_000_000;
    let mut waits = Vec::new();
    for _ in 0..5 {
        let r = record_at(&conn, &locked, at).unwrap();
        assert_eq!((r.class, r.scope), (Transient, Item));
        let next = r.next_at.unwrap();
        waits.push((r.attempt, (next - at) / MINUTE));
        at = next;
    }
    assert_eq!(waits, vec![(1, 10), (2, 60), (3, 360), (4, 1440), (5, 1440)]);
    let (state, file, diagnose): (String, Option<String>, bool) = conn
        .query_row("SELECT state, file_hash, to_diagnose FROM failure WHERE stage = 'shapes'", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap();
    assert_eq!((state.as_str(), file.as_deref(), diagnose), ("open", Some("h1"), false));

    // Permanent and unknown: no next attempt, waiting for a person, to
    // diagnose, and the indicator at once.
    for (stage, message) in [("cut", "PdfiumLibraryInternalError(PasswordError)"), ("parts", "algo raro")] {
        let r = record_at(&conn, &Failure::new(stage, About::File("h1".into()), message), at).unwrap();
        assert_eq!(r.next_at, None);
        let (state, indicator, diagnose): (String, bool, bool) = conn
            .query_row("SELECT state, indicator, to_diagnose FROM failure WHERE id = ?1", [r.id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .unwrap();
        assert_eq!((state.as_str(), indicator, diagnose), ("person", true, true));
    }

    // What failed is not done again over the same basis while it holds;
    // a transient one, once its next attempt comes.
    let mut failure = Failure::new("cut", About::File("h1".into()), "database is locked");
    failure.basis = Some("r|5");
    conn.execute("DELETE FROM failure", []).unwrap();
    record_at(&conn, &failure, super::super::now_ms()).unwrap();
    let holds = |basis: &str| -> bool {
        conn.query_row(&format!("SELECT {}", holds_sql("cut", "'h1'", &format!("'{basis}'"))), [], |r| r.get(0)).unwrap()
    };
    assert!(holds("r|5"));
    assert!(!holds("r|6"), "another text or other rules: done again");
    conn.execute("UPDATE failure SET next_at = 0", []).unwrap();
    assert!(!holds("r|5"), "its next attempt came");
    // And it is resolved when the work comes out well.
    resolve(&conn, "cut", &About::File("h1".into())).unwrap();
    let open: i64 = conn.query_row("SELECT count(*) FROM failure WHERE state <> 'resolved'", [], |r| r.get(0)).unwrap();
    assert_eq!(open, 0);
}

#[test]
fn the_indicator_comes_past_the_threshold_of_attempts_or_time_of_the_settings() {
    let conn = base();
    let busy = Failure::new("cut", About::File("h1".into()), "database is locked");
    let indicator = |id: i64| -> bool { conn.query_row("SELECT indicator FROM failure WHERE id = ?1", [id], |r| r.get(0)).unwrap() };
    let first = record_at(&conn, &busy, 0).unwrap();
    assert!(!indicator(first.id));
    record_at(&conn, &busy, MINUTE).unwrap();
    assert!(!indicator(first.id));
    // The third attempt: the threshold by default.
    record_at(&conn, &busy, 2 * MINUTE).unwrap();
    assert!(indicator(first.id));

    // By time: 6 hours by default; the setting changes it.
    let other = Failure::new("parts", About::File("h1".into()), "database is locked");
    let r = record_at(&conn, &other, 0).unwrap();
    record_at(&conn, &other, 5 * 60 * MINUTE).unwrap();
    assert!(!indicator(r.id));
    conn.execute("INSERT INTO docbase_setting (key, value) VALUES (?1, '4')", [INDICATOR_HOURS]).unwrap();
    conn.execute("INSERT INTO docbase_setting (key, value) VALUES (?1, '10')", [INDICATOR_ATTEMPTS]).unwrap();
    assert_eq!(indicator_threshold(&conn).unwrap(), (10, 4));
    record_at(&conn, &other, 5 * 60 * MINUTE + 1).unwrap();
    assert!(indicator(r.id));

    // The disk full, at once.
    let full = record_at(&conn, &Failure::new("similar", About::Version(1), "database or disk is full"), 0).unwrap();
    assert!(indicator(full.id));
}

#[test]
fn an_answer_of_another_shape_turns_unknown_at_the_third_equal_one() {
    let conn = base();
    let span = About::Span { file: "h1".into(), after: 2 };
    let shaped = |answer: &'static str| {
        let mut f = Failure::new(CUT_QUESTION_STAGE, span.clone(), "The answer is not of the shape asked");
        f.cause = cause("bad-answer");
        f.engine = Some("falso:1");
        f.answer = Some(answer);
        f
    };
    // Two equal, then another one: it counts again from one.
    assert_eq!(record_at(&conn, &shaped("{\"x\": 1}"), 0).unwrap().class, Transient);
    assert_eq!(record_at(&conn, &shaped("{\"x\": 1}"), 1).unwrap().class, Transient);
    assert_eq!(record_at(&conn, &shaped("{\"y\": 2}"), 2).unwrap().class, Transient);
    assert_eq!(record_at(&conn, &shaped("{\"y\": 2}"), 3).unwrap().class, Transient);
    // The third equal: unknown, not tried again, to diagnose.
    let r = record_at(&conn, &shaped(" {\"y\": 2} "), 4).unwrap();
    assert_eq!((r.class, r.next_at, r.attempt), (Unknown, None, 5));
    let (state, diagnose): (String, bool) =
        conn.query_row("SELECT state, to_diagnose FROM failure WHERE id = ?1", [r.id], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
    assert_eq!((state.as_str(), diagnose), ("person", true));
}

#[test]
fn a_failure_of_the_whole_engine_pauses_only_that_engine_until_its_next_attempt() {
    let conn = base();
    let mut limit = Failure::new("types", About::Engine("motor:a".into()), "429 Too Many Requests");
    limit.engine = Some("motor:a");
    let r = record_at(&conn, &limit, 1_000).unwrap();
    assert_eq!(engine_paused_at(&conn, "motor:a", 2_000).unwrap(), r.next_at);
    assert_eq!(engine_paused_at(&conn, "motor:b", 2_000).unwrap(), None);
    assert_eq!(engine_paused_at(&conn, "motor:a", r.next_at.unwrap()).unwrap(), None, "its next attempt came");
    // An engine that does not look at images does not pause the others'
    // questions: its failure is of the question of the cut alone.
    let mut blind = Failure::new(CUT_QUESTION_STAGE, About::Engine("motor:c".into()), "does not look at images");
    blind.cause = cause("engine-no-images");
    record_at(&conn, &blind, 1_000).unwrap();
    assert_eq!(engine_paused_at(&conn, "motor:c", 2_000).unwrap(), None);
    // The credential: until a person acts.
    let key = Failure::new("reading", About::Engine("motor:d".into()), "401 Unauthorized");
    record_at(&conn, &key, 1_000).unwrap();
    assert_eq!(engine_paused_at(&conn, "motor:d", i64::MAX - 1).unwrap(), Some(i64::MAX));
    resolve_engine(&conn, "motor:d").unwrap();
    assert_eq!(engine_paused_at(&conn, "motor:d", 2_000).unwrap(), None);
}

#[test]
fn the_keys_failed_of_the_settings_come_into_the_table_and_are_tried_again_to_learn_their_cause() {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    super::super::schema::build_at(&mut conn, 7);
    conn.execute_batch(
        "INSERT INTO version (id, state, first_seen_at) VALUES (1, 'processed', 1);
         INSERT INTO file (hash, version_id, size) VALUES ('h1', 1, 10);
         INSERT INTO docbase_setting (key, value) VALUES
             ('failed:cut:h1', 'cut-rules/2+cut.json/1|5'), ('failed:parts:h1', '1:5:6::'),
             ('failed:similar:1', '5|sim'), ('failed:zones:gone', 'v9'), ('background-reading', 'on');",
    )
    .unwrap();
    super::super::schema::migrate(&mut conn).unwrap();
    let rows: Vec<(String, String, String, Option<String>, Option<i64>, String, String, String, bool)> = conn
        .prepare(
            "SELECT stage, item_kind, item, file_hash, version_id, basis, cause, class, next_at IS NOT NULL FROM failure ORDER BY stage",
        )
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let s = |v: &str| v.to_string();
    assert_eq!(
        rows,
        vec![
            (s("cut"), s("file"), s("h1"), Some(s("h1")), None, s("cut-rules/2+cut.json/1|5"), s("before-record"), s("transient"), true),
            (s("parts"), s("file"), s("h1"), Some(s("h1")), None, s("1:5:6::"), s("before-record"), s("transient"), true),
            (s("similar"), s("version"), s("1"), None, Some(1), s("5|sim"), s("before-record"), s("transient"), true),
            // A file no longer in the base keeps its row, without the link.
            (s("zones"), s("file"), s("gone"), None, None, s("v9"), s("before-record"), s("transient"), true),
        ]
    );
    let left: i64 = conn.query_row("SELECT count(*) FROM docbase_setting WHERE key LIKE 'failed:%'", [], |r| r.get(0)).unwrap();
    assert_eq!(left, 0, "the keys are not kept");
    let kept: i64 = conn.query_row("SELECT count(*) FROM docbase_setting WHERE key = 'background-reading'", [], |r| r.get(0)).unwrap();
    assert_eq!(kept, 1, "the other settings stay");
    // Due now: the work is done again, and its real cause written down.
    let held: bool =
        conn.query_row(&format!("SELECT {}", holds_sql("cut", "'h1'", "'cut-rules/2+cut.json/1|5'")), [], |r| r.get(0)).unwrap();
    assert!(!held);
    // Without the file, its failures go.
    conn.execute("DELETE FROM file WHERE hash = 'h1'", []).unwrap();
    let left: i64 = conn.query_row("SELECT count(*) FROM failure WHERE item = 'h1'", [], |r| r.get(0)).unwrap();
    assert_eq!(left, 0);
}

/// Opens a copy of a base with this version (`MICELYA_FAILURES_COPY`),
/// for the check of the first stage of 1 nov: the migration runs, and
/// nothing else. Says what waits under the new rules.
#[test]
#[ignore]
fn open_a_copy_of_a_base() {
    let Ok(path) = std::env::var("MICELYA_FAILURES_COPY") else { return };
    let base = crate::docbase::DocBase::open(std::path::Path::new(&path)).unwrap();
    println!(
        "waiting for the cut: {}; waiting for answers about the cut: {}; waiting for the signature: {}; open failures: {}",
        base.waiting_for_cut().unwrap().len(),
        base.waiting_for_cut_answers(true).unwrap().len(),
        base.waiting_for_similar().unwrap().len(),
        base.failures_open().unwrap().len()
    );
}

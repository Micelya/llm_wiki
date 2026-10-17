//! How the resolution of identities reads a name: its words without the
//! ones that do not count (articles, titles), the legal form of an
//! organization, a mail address written as a name; and how two names
//! compare (D-186).
//!
//! Everything here is a computation over the words of the vocabulary of
//! the layer (`identity` in `knowledge.json`, R-26): no name, no case.

use std::collections::BTreeSet;

use super::quote;
use super::vocabulary::Vocabulary;

/// A name as the resolution compares it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NameKey {
    /// The words that count, in the order written (without articles,
    /// titles and, for an organization, its legal form).
    pub words: Vec<String>,
    /// The legal form of an organization, when the name says it.
    pub form: Option<String>,
    /// A mail address written as the name (or inside it).
    pub email: Option<String>,
    /// All the words that count, the legal form's included (for an
    /// acronym that takes the form in: "MEA" of "Mundo Ejemplo Association").
    pub full: Vec<String>,
}

impl NameKey {
    /// Whether there is a name to compare at all (a phone number or a
    /// code is not one).
    pub fn has_name(&self) -> bool {
        self.words.iter().any(|w| w.chars().any(char::is_alphabetic))
    }

    /// The words in order, for showing and for keys.
    pub fn joined(&self) -> String {
        self.words.join(" ")
    }

    /// The words without their order: a person's names and surnames
    /// come in either order.
    pub fn sorted(&self) -> Vec<String> {
        let mut words = self.words.clone();
        words.sort();
        words
    }

    /// The key of an exact match: the same words (in any order for a
    /// person) and the same legal form.
    pub fn exact_key(&self, kind: &str) -> String {
        let words = if kind == PERSON { self.sorted().join(" ") } else { self.joined() };
        format!("{kind}|{words}|{}", self.form.as_deref().unwrap_or(""))
    }

    /// How specific the name is, to choose the principal name of a group
    /// of mentions: a name before a number or an address, more words
    /// with letters first, then a legal form.
    pub fn weight(&self) -> (bool, usize, bool) {
        let lettered = self.words.iter().filter(|w| w.chars().any(char::is_alphabetic)).count();
        (lettered > 0, lettered, self.form.is_some())
    }
}

pub const PERSON: &str = "persona";
pub const ORGANIZATION: &str = "organizacion";

/// The mail address in a written name, if any.
fn email_in(written: &str) -> Option<String> {
    written
        .split(|c: char| c.is_whitespace() || "()<>[],;:\"'«»".contains(c))
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()))
        .find(|t| {
            let Some((user, domain)) = t.split_once('@') else { return false };
            !user.is_empty() && domain.contains('.')
        })
        .map(|t| t.to_lowercase())
}

/// The text without what is between parentheses, unless that is all.
fn without_parentheses(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    for c in text.chars() {
        match c {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    if quote::words(&out).is_empty() {
        text.to_string()
    } else {
        out
    }
}

/// Reads a name of an entity of this kind.
pub fn key(vocabulary: &Vocabulary, kind: &str, written: &str) -> NameKey {
    let identity = &vocabulary.identity;
    let email = email_in(written);
    let rest = match &email {
        Some(address) => {
            // The words of the address are not the name.
            let lower = written.to_lowercase();
            lower.replace(address.as_str(), " ")
        }
        None => written.to_string(),
    };
    // What is in parentheses complements the name (an acronym, the
    // company behind a brand): it is not the name.
    let rest = without_parentheses(&rest);
    let mut words: Vec<String> = quote::words(&rest);
    let stop: BTreeSet<&str> = identity.stopwords.iter().map(String::as_str).collect();
    let titles: BTreeSet<&str> = identity.titles.iter().map(String::as_str).collect();
    let counts = |w: &String| !stop.contains(w.as_str()) && !(kind == PERSON && titles.contains(w.as_str()));
    let full: Vec<String> = words.iter().filter(|w| counts(w)).cloned().collect();
    let mut form = None;
    if kind == ORGANIZATION {
        form = take_form(vocabulary, &mut words);
    }
    words.retain(counts);
    // Words that name someone without being a name ("el cliente", "mi",
    // "la locataria"): no name to compare (D-211).
    let generic = |w: &String| identity.generic.iter().any(|g| g == w) || titles.contains(w.as_str());
    // A matter is often named so ("el acuerdo"): it is still a mention of
    // it, joined only by its identifiers or by the model.
    if kind != "asunto" && form.is_none() && !words.is_empty() && words.iter().all(generic) {
        return NameKey { words: Vec::new(), form, email, full: Vec::new() };
    }
    NameKey { words, form, email, full }
}

/// Takes the legal form off the end of the words (or off the start, for
/// the forms written before the name), the longest way of writing it first.
fn take_form(vocabulary: &Vocabulary, words: &mut Vec<String>) -> Option<String> {
    let mut variants: Vec<(Vec<String>, &str, bool)> = vocabulary
        .identity
        .legal_forms
        .iter()
        .flat_map(|f| f.variants.iter().map(move |v| (quote::words(v), f.id.as_str(), f.before)))
        .filter(|(v, _, _)| !v.is_empty())
        .collect();
    variants.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.1.cmp(b.1)));
    for (variant, id, _) in &variants {
        if words.len() > variant.len() && words.ends_with(variant) {
            words.truncate(words.len() - variant.len());
            return Some(id.to_string());
        }
    }
    for (variant, id, before) in &variants {
        if *before && words.len() > variant.len() && words.starts_with(variant) {
            words.drain(..variant.len());
            return Some(id.to_string());
        }
    }
    None
}

/// How two names of the same kind compare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Likeness {
    /// The same name: joins without asking, when nothing keeps them apart.
    Exact,
    /// The same name of an organization with another legal form: another
    /// organization.
    OtherForm,
    /// It may be the same: the model is asked, with the reason why.
    Maybe(&'static str),
    /// Nothing in common.
    Unlike,
}

/// The edit distance between two words, up to `limit` (more is `limit + 1`).
pub fn distance(a: &str, b: &str, limit: usize) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > limit {
        return limit + 1;
    }
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut current = vec![i + 1; b.len() + 1];
        let mut best = current[0];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            current[j + 1] = (previous[j] + cost).min(previous[j + 1] + 1).min(current[j] + 1);
            best = best.min(current[j + 1]);
        }
        if best > limit {
            return limit + 1;
        }
        previous = current;
    }
    previous[b.len()].min(limit + 1)
}

/// Whether two words differ only by a typing or reading error: one letter
/// in a word of four or more, two in a word of seven or more.
pub fn typo(a: &str, b: &str) -> bool {
    let shortest = a.chars().count().min(b.chars().count());
    let limit = if shortest >= 7 { 2 } else if shortest >= 4 { 1 } else { 0 };
    limit > 0 && a != b && distance(a, b, limit) <= limit
}

/// How the words of `short` meet those of `long`, each word of `short`
/// with a different word of `long`: `None` when one does not meet any;
/// otherwise (equal words, initials, typos).
fn align(short: &[String], long: &[String]) -> Option<(usize, usize, usize)> {
    let mut used = vec![false; long.len()];
    let (mut equal, mut initials, mut typos) = (0, 0, 0);
    // Equal words first, so an initial or a typo never takes a word that
    // another one matches exactly.
    let mut pending: Vec<&String> = Vec::new();
    for word in short {
        match (0..long.len()).find(|&j| !used[j] && &long[j] == word) {
            Some(j) => {
                used[j] = true;
                equal += 1;
            }
            None => pending.push(word),
        }
    }
    for word in pending {
        let initial = |a: &str, b: &str| a.chars().count() == 1 && b.starts_with(a);
        if let Some(j) = (0..long.len()).find(|&j| !used[j] && (initial(word, &long[j]) || initial(&long[j], word))) {
            used[j] = true;
            initials += 1;
            continue;
        }
        if let Some(j) = (0..long.len()).find(|&j| !used[j] && typo(word, &long[j])) {
            used[j] = true;
            typos += 1;
            continue;
        }
        return None;
    }
    Some((equal, initials, typos))
}

/// How two names of people compare. The order of the words does not
/// count; a name of one word is never enough to join by itself.
fn people(a: &NameKey, b: &NameKey) -> Likeness {
    let (sa, sb) = (a.sorted(), b.sorted());
    if sa.is_empty() || sb.is_empty() {
        return Likeness::Unlike;
    }
    if sa == sb {
        return if sa.len() >= 2 { Likeness::Exact } else { Likeness::Maybe("una-palabra") };
    }
    let (short, long) = if sa.len() <= sb.len() { (&sa, &sb) } else { (&sb, &sa) };
    let Some((equal, initials, typos)) = align(short, long) else {
        return Likeness::Unlike;
    };
    // At least one whole word in common, besides initials and typos.
    if equal == 0 || (short.len() == 1 && short[0].chars().count() < 3) {
        return Likeness::Unlike;
    }
    if short.len() == 1 {
        return Likeness::Maybe("una-palabra");
    }
    if typos > 0 {
        return Likeness::Maybe("letra-distinta");
    }
    if initials > 0 {
        return Likeness::Maybe("inicial");
    }
    Likeness::Maybe("nombre-parcial")
}

/// Whether two lists of words are the same but for one word, and those
/// two start with the same four letters (a word misspelled further on).
fn one_word_apart_by_its_start(a: &[String], b: &[String]) -> bool {
    let different: Vec<(&String, &String)> = a.iter().zip(b).filter(|(x, y)| x != y).collect();
    different.len() == 1 && {
        let (x, y) = different[0];
        x.chars().count().min(y.chars().count()) >= 5 && x.chars().take(4).eq(y.chars().take(4))
    }
}

/// The initials of the words, for an acronym.
fn initials(words: &[String]) -> String {
    words.iter().filter_map(|w| w.chars().next()).collect()
}

/// How two names of organizations compare: the words in order, and the
/// legal form apart.
fn organizations(a: &NameKey, b: &NameKey) -> Likeness {
    if a.words.is_empty() || b.words.is_empty() {
        return Likeness::Unlike;
    }
    let forms_differ = matches!((&a.form, &b.form), (Some(x), Some(y)) if x != y);
    let likeness = if a.words == b.words {
        if a.form == b.form {
            if a.words.len() >= 2 || a.form.is_some() {
                Likeness::Exact
            } else {
                Likeness::Maybe("una-palabra")
            }
        } else {
            Likeness::Maybe("sin-forma")
        }
    } else {
        let (short, long) = if a.words.len() <= b.words.len() { (&a.words, &b.words) } else { (&b.words, &a.words) };
        let aligned = align(short, long);
        let acronym = |one: &[String], other: &[String]| {
            one.len() == 1 && other.len() >= 2 && one[0].chars().count() >= 2 && one[0] == initials(other)
        };
        if short.len() < long.len() && aligned.is_some_and(|(equal, initials, _)| equal > 0 && initials == 0) {
            Likeness::Maybe("nombre-parcial")
        } else if short.len() == long.len() && aligned.is_some_and(|(equal, initials, _)| initials == 0 && (equal > 0 || short.len() == 1)) {
            Likeness::Maybe("letra-distinta")
        } else if short.len() == long.len() && a.form.is_some() && a.form == b.form && one_word_apart_by_its_start(short, long) {
            Likeness::Maybe("letra-distinta")
        } else if acronym(short, long) || acronym(&a.words, &b.full) || acronym(&b.words, &a.full) {
            Likeness::Maybe("sigla")
        } else {
            Likeness::Unlike
        }
    };
    if forms_differ && likeness != Likeness::Unlike {
        return Likeness::OtherForm;
    }
    likeness
}

/// How two names of matters compare. A matter is never joined by its name
/// alone: the same name ("el acuerdo") is often another one.
fn matters(a: &NameKey, b: &NameKey) -> Likeness {
    if a.words.is_empty() || b.words.is_empty() {
        return Likeness::Unlike;
    }
    if a.words == b.words {
        return Likeness::Maybe("mismo-nombre");
    }
    let (short, long) = if a.words.len() <= b.words.len() { (&a.words, &b.words) } else { (&b.words, &a.words) };
    let set: BTreeSet<&String> = long.iter().collect();
    if short.len() >= 2 && short.iter().all(|w| set.contains(w)) {
        return Likeness::Maybe("nombre-parcial");
    }
    Likeness::Unlike
}

/// An address with no name against a name: it may be the same when a
/// word of the name (four letters or more) is in the user part of the
/// address.
fn by_address(lone: &NameKey, named: &NameKey) -> Option<Likeness> {
    let address = lone.email.as_ref().filter(|_| !lone.has_name())?;
    if !named.has_name() {
        return None;
    }
    let user: String = address.split('@').next().unwrap_or_default().chars().filter(|c| c.is_alphanumeric()).collect();
    named
        .words
        .iter()
        .any(|w| w.chars().count() >= 4 && w.chars().all(char::is_alphanumeric) && user.contains(w.as_str()))
        .then_some(Likeness::Maybe("casilla"))
}

/// The user part of an address with no name, cut in every piece of five
/// letters or more, to be found by the words of a name.
fn address_pieces(key: &NameKey) -> Vec<String> {
    let Some(address) = key.email.as_ref().filter(|_| !key.has_name()) else { return Vec::new() };
    let user: Vec<char> = address.split('@').next().unwrap_or_default().chars().filter(|c| c.is_alphanumeric()).collect();
    let mut out = Vec::new();
    for start in 0..user.len() {
        for end in start + 5..=user.len().min(start + 24) {
            out.push(user[start..end].iter().collect());
        }
    }
    out
}

/// How two names of entities of the same kind compare.
pub fn compare(kind: &str, a: &NameKey, b: &NameKey) -> Likeness {
    if let (Some(x), Some(y)) = (&a.email, &b.email) {
        if x == y {
            // The same address: the same, unless the names say otherwise.
            // The same address: the same when only one side has a name
            // or both names are the same; otherwise a shared mailbox (a
            // legal department's) may be several: the model is asked.
            if !a.has_name() || !b.has_name() {
                return Likeness::Exact;
            }
            return match compare(kind, &NameKey { email: None, ..a.clone() }, &NameKey { email: None, ..b.clone() }) {
                Likeness::Exact => Likeness::Exact,
                Likeness::OtherForm => Likeness::OtherForm,
                _ => Likeness::Maybe("misma-casilla"),
            };
        }
    }
    if !a.has_name() || !b.has_name() {
        return by_address(a, b).or_else(|| by_address(b, a)).unwrap_or(Likeness::Unlike);
    }
    match kind {
        PERSON => people(a, b),
        ORGANIZATION => organizations(a, b),
        _ => matters(a, b),
    }
}

/// The keys a name is found by when looking for candidates: its words,
/// the first letters of its longer words (a typo further on), its
/// initials (an acronym) and its mail address. Two names are compared
/// only when they share one of these (no comparison of all against all).
pub fn block_keys(key: &NameKey) -> Vec<String> {
    let mut out: BTreeSet<String> = BTreeSet::new();
    for word in &key.words {
        if word.chars().count() >= 2 {
            out.insert(format!("w:{word}"));
        }
        if word.chars().count() >= 4 {
            out.insert(format!("p:{}", word.chars().take(3).collect::<String>()));
        }
    }
    if key.words.len() == 1 {
        out.insert(format!("a:{}", key.words[0]));
    } else if key.words.len() >= 2 {
        out.insert(format!("a:{}", initials(&key.words)));
    }
    if key.full.len() >= 2 {
        out.insert(format!("a:{}", initials(&key.full)));
    }
    if let Some(email) = &key.email {
        out.insert(format!("e:{email}"));
    }
    // An address with no name is found by the words of a name it holds.
    for word in &key.words {
        if word.chars().count() >= 5 && word.chars().all(char::is_alphanumeric) {
            out.insert(format!("s:{word}"));
        }
    }
    for piece in address_pieces(key) {
        out.insert(format!("s:{piece}"));
    }
    out.into_iter().collect()
}

/// An identifier as compared: letters and digits only, lowercase.
pub fn identifier_value(value: &str) -> String {
    value.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// Whether two different identifiers of the same kind may be one misread
/// (a scanned digit): one character of difference in values of six or more.
pub fn misread(a: &str, b: &str) -> bool {
    a != b && a.chars().count().min(b.chars().count()) >= 6 && distance(a, b, 1) <= 1
}

/// The name an entity is shown with: on one line, and a name written all in
/// capitals ("PERSONA DOS, UNO") in the form of a proper name when the
/// documents write each of its words some other way ("Uno Persona Dos"):
/// its order and punctuation, the case of the other forms (D-239). A word
/// of two letters or more that no other form writes leaves it as it is.
pub fn shown_name<'a>(name: &str, forms: impl IntoIterator<Item = &'a str>) -> String {
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let letters: Vec<char> = name.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.len() < 2 || letters.iter().any(|c| c.is_lowercase()) {
        return name;
    }
    let fold = |word: &str| -> String { quote::words(word).join("") };
    // Each word as the other forms write it, when they do not shout it.
    let mut cased: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for form in forms {
        if !form.chars().any(|c| c.is_lowercase()) {
            continue;
        }
        for word in form.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()) {
            let key = fold(word);
            let mixed = word.chars().any(|c| c.is_lowercase());
            match cased.get(&key) {
                Some(seen) if seen.chars().any(|c| c.is_lowercase()) || !mixed => {}
                _ => {
                    cased.insert(key, word.to_string());
                }
            }
        }
    }
    let mut out = String::new();
    let mut word = String::new();
    let mut complete = true;
    let flush = |word: &mut String, out: &mut String, complete: &mut bool| {
        if word.is_empty() {
            return;
        }
        match cased.get(&fold(word)) {
            Some(form) => out.push_str(form),
            None => {
                if word.chars().filter(|c| c.is_alphabetic()).count() >= 2 {
                    *complete = false;
                }
                out.push_str(word);
            }
        }
        word.clear();
    };
    for c in name.chars() {
        if c.is_alphanumeric() {
            word.push(c);
        } else {
            flush(&mut word, &mut out, &mut complete);
            out.push(c);
        }
    }
    flush(&mut word, &mut out, &mut complete);
    if complete {
        out
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_in_capitals_takes_the_case_the_other_forms_write() {
        assert_eq!(shown_name("PERSONA DOS, UNO", ["Uno Persona Dos", "PERSONA DOS, UNO"]), "Persona Dos, Uno");
        assert_eq!(shown_name("EJEMPLO UNO S.A.", ["Ejemplo Uno S.A."]), "Ejemplo Uno S.A.");
        assert_eq!(shown_name("PEREZ, ANA", ["Ana Pérez"]), "Pérez, Ana");
        // A word no other form writes: as it is.
        assert_eq!(shown_name("PERSONA TRES CUATRO", ["Persona Tres"]), "PERSONA TRES CUATRO");
        // Not in capitals, or no other form: as it is, on one line.
        assert_eq!(shown_name("Persona\n Dos", ["PERSONA DOS"]), "Persona Dos");
        assert_eq!(shown_name("ACME", Vec::<&str>::new()), "ACME");
    }

    fn k(kind: &str, name: &str) -> NameKey {
        key(&Vocabulary::builtin(), kind, name)
    }

    #[test]
    fn a_name_loses_its_articles_titles_and_legal_form() {
        assert_eq!(k(PERSON, "Dr. Persona de la Uno").words, ["persona", "uno"]);
        let company = k(ORGANIZATION, "Ejemplo Uno S.A.");
        assert_eq!((company.words.join(" "), company.form.as_deref()), ("ejemplo uno".into(), Some("sa")));
        assert_eq!(k(ORGANIZATION, "EJEMPLO UNO SA").form.as_deref(), Some("sa"));
        assert_eq!(k(ORGANIZATION, "Ejemplo Uno Pte. Ltd.").form.as_deref(), Some("pte-ltd"));
        assert_eq!(k(ORGANIZATION, "Ejemplo Uno Ltd").form.as_deref(), Some("ltd"));
        let association = k(ORGANIZATION, "Verein Ejemplo Uno");
        assert_eq!((association.joined(), association.form.as_deref()), ("ejemplo uno".into(), Some("asociacion")));
        // A form alone is not taken off: it is the whole name.
        assert_eq!(k(ORGANIZATION, "S.A.").form, None);
        let mail = k(PERSON, "Persona Uno <puno@ejemplo.com>");
        assert_eq!((mail.joined(), mail.email.as_deref()), ("persona uno".into(), Some("puno@ejemplo.com")));
        assert!(!k(PERSON, "11 2222 3333").has_name());
        assert_eq!(k(ORGANIZATION, "Marca (Ejemplo Uno S.A.)").joined(), "marca");
        assert_eq!(k(ORGANIZATION, "(Ejemplo)").joined(), "ejemplo");
    }

    #[test]
    fn people_compare_in_any_order_with_initials_and_typos_as_doubts() {
        let same = |a: &str, b: &str| compare(PERSON, &k(PERSON, a), &k(PERSON, b));
        assert_eq!(same("Persona Uno Ejemplo", "EJEMPLO, Persona Uno"), Likeness::Exact);
        assert_eq!(same("Persona Ejemplo", "Persona Uno Ejemplo"), Likeness::Maybe("nombre-parcial"));
        assert_eq!(same("Persona U. Ejemplo", "Persona Uno Ejemplo"), Likeness::Maybe("inicial"));
        assert_eq!(same("Persona Ejemplo", "Persona Ejenplo"), Likeness::Maybe("letra-distinta"));
        assert_eq!(same("Persona", "Persona Ejemplo"), Likeness::Maybe("una-palabra"));
        assert_eq!(same("Persona", "Persona"), Likeness::Maybe("una-palabra"));
        // A first name in common is not a likeness; nor is an initial alone.
        assert_eq!(same("Persona Ejemplo", "Persona Otro"), Likeness::Unlike);
        assert_eq!(same("P. Ejemplo", "Otra Persona"), Likeness::Unlike);
    }

    #[test]
    fn organizations_with_another_form_are_others() {
        let same = |a: &str, b: &str| compare(ORGANIZATION, &k(ORGANIZATION, a), &k(ORGANIZATION, b));
        assert_eq!(same("Ejemplo Uno S.A.", "EJEMPLO UNO SA"), Likeness::Exact);
        assert_eq!(same("Ejemplo Uno S.A.", "Ejemplo Uno LLC"), Likeness::OtherForm);
        assert_eq!(same("Ejemplo Uno S.A.", "Ejemplo Uno"), Likeness::Maybe("sin-forma"));
        assert_eq!(same("Ejemplo", "Ejemplo Uno S.A."), Likeness::Maybe("nombre-parcial"));
        assert_eq!(same("Ejemplo Uno S.A.", "Ejempla Uno S.A."), Likeness::Maybe("letra-distinta"));
        assert_eq!(same("CEU", "Consejo Ejemplo Uno"), Likeness::Maybe("sigla"));
        assert_eq!(same("Ejemplo", "Ejemplo"), Likeness::Maybe("una-palabra"));
        assert_eq!(same("Grupo Ejemplo", "Ejemplo Uno S.A."), Likeness::Unlike);
        // A partial name with a typo, an acronym of the whole name, a
        // word misspelled further on.
        assert_eq!(same("Correo Ejemplar", "Correo Oficial del Ejemplor"), Likeness::Maybe("nombre-parcial"));
        assert_eq!(same("MEA", "Mundo Ejemplo Association"), Likeness::Maybe("sigla"));
        assert_eq!(same("Ejemplonia S.A.", "Ejemplo S.A."), Likeness::Maybe("letra-distinta"));
    }

    #[test]
    fn a_matter_is_never_joined_by_its_name_alone() {
        let same = |a: &str, b: &str| compare("asunto", &k("asunto", a), &k("asunto", b));
        assert_eq!(same("Acuerdo", "acuerdo"), Likeness::Maybe("mismo-nombre"));
        assert_eq!(same("Acuerdo", "Acuerdo de terminación"), Likeness::Unlike);
        assert_eq!(same("Acuerdo de terminación", "Acuerdo de terminación anticipada"), Likeness::Maybe("nombre-parcial"));
    }

    #[test]
    fn the_same_address_is_the_same_unless_the_names_differ() {
        let same = |a: &str, b: &str| compare(PERSON, &k(PERSON, a), &k(PERSON, b));
        assert_eq!(same("puno@ejemplo.com", "Persona Uno (puno@ejemplo.com)"), Likeness::Exact);
        assert_eq!(same("Persona Uno (puno@ejemplo.com)", "Otra Gente <puno@ejemplo.com>"), Likeness::Maybe("misma-casilla"));
        assert_eq!(same("Persona Uno (puno@ejemplo.com)", "Persona <puno@ejemplo.com>"), Likeness::Maybe("misma-casilla"));
        assert_eq!(same("puno@ejemplo.com", "pdos@ejemplo.com"), Likeness::Unlike);
        // An address with no name, against a name it holds.
        assert_eq!(same("pejemplar@ejemplo.com", "Persona Ejemplar"), Likeness::Maybe("casilla"));
        assert!(block_keys(&k(PERSON, "pejemplar@ejemplo.com")).contains(&"s:ejemplar".to_string()));
        assert!(block_keys(&k(PERSON, "Persona Ejemplar")).contains(&"s:ejemplar".to_string()));
    }

    #[test]
    fn identifiers_compare_by_letters_and_digits() {
        assert_eq!(identifier_value("30-11111111-1"), "30111111111");
        assert_eq!(identifier_value("22.222.222"), "22222222");
        assert!(misread("22222222", "22222223"));
        assert!(!misread("22222222", "22223333"));
        assert!(!misread("12345", "12346"), "too short to tell");
        assert_eq!(distance("ejemplo", "ejenplo", 2), 1);
        assert!(typo("ejemplo", "ejemplp"));
        assert!(!typo("uno", "una"));
    }

    #[test]
    fn names_are_found_by_their_words_initials_and_address() {
        let keys = block_keys(&k(ORGANIZATION, "Consejo Ejemplo Uno"));
        assert!(keys.contains(&"w:consejo".to_string()));
        assert!(keys.contains(&"p:con".to_string()));
        assert!(keys.contains(&"a:ceu".to_string()));
        assert!(block_keys(&k(ORGANIZATION, "CEU")).contains(&"a:ceu".to_string()));
    }
}

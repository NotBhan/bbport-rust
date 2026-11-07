//! Launcher translations, ported from `bbport_i18n.py`. The catalogue is generated from the
//! Python module into `i18n_catalog.json` and embedded in the binary.

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// Languages the launcher offers, in the Python order.
pub const LANGUAGES: &[&str] = &["en", "fr", "ru", "zh", "ko", "ja"];

#[derive(Deserialize)]
struct Catalog {
    en: HashMap<String, String>,
    ru: HashMap<String, String>,
    catalogs: HashMap<String, HashMap<String, String>>,
}

fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("bbport_i18n_catalog.json"))
            .expect("the embedded i18n catalogue is valid JSON")
    })
}

fn current() -> &'static RwLock<String> {
    static CURRENT: OnceLock<RwLock<String>> = OnceLock::new();
    CURRENT.get_or_init(|| RwLock::new("en".to_string()))
}

/// Detects the system language from the environment, defaulting to `en`.
pub fn system_language() -> String {
    for key in ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
        let Ok(value) = std::env::var(key) else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        let lowered = value.to_lowercase();
        for (prefix, language) in [
            ("fr", "fr"),
            ("ru", "ru"),
            ("zh", "zh"),
            ("ko", "ko"),
            ("ja", "ja"),
        ] {
            if lowered.starts_with(prefix) {
                return language.to_string();
            }
        }
        return "en".to_string();
    }
    "en".to_string()
}

/// `choice`: `en`, `fr`, `ru`, `zh`, `ko`, `ja`, or empty/`None` for the system language.
pub fn set_language(choice: Option<&str>) -> String {
    let language = match choice {
        Some(value) if LANGUAGES.contains(&value) => value.to_string(),
        Some("") | None => system_language(),
        Some(_) => "en".to_string(),
    };
    if let Ok(mut guard) = current().write() {
        *guard = language.clone();
    }
    language
}

pub fn language() -> String {
    current()
        .read()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| "en".into())
}

/// Translates `text` into `lang` without changing the launcher's current language.
pub fn translate(text: &str, lang: Option<&str>) -> String {
    let Some(lang) = lang else {
        return tr(text);
    };
    let previous = language();
    let result = {
        set_language(Some(lang));
        tr(text)
    };
    set_language(Some(&previous));
    result
}

/// Translates `text` into the current language, falling back to English.
pub fn tr(text: &str) -> String {
    if text.is_empty() {
        return text.to_string();
    }
    let tables = catalog();
    let is_russian_source = text
        .chars()
        .any(|character| ('\u{0400}'..='\u{04FF}').contains(&character))
        && tables.en.contains_key(text);
    let english = tables
        .en
        .get(text)
        .cloned()
        .unwrap_or_else(|| text.to_string());

    let current_language = language();
    if current_language == "en" {
        return english;
    }
    if current_language == "ru" {
        if is_russian_source {
            return text.to_string();
        }
        return tables.ru.get(&english).cloned().unwrap_or(english);
    }
    tables
        .catalogs
        .get(&current_language)
        .and_then(|catalog| catalog.get(&english))
        .cloned()
        .unwrap_or(english)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// The current language is process-global, like the Python module's `_language`.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn english_is_the_fallback() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        set_language(Some("en"));
        assert_eq!(tr("Launch Game"), "Launch Game");
        assert_eq!(
            tr("something with no translation"),
            "something with no translation"
        );
    }

    #[test]
    fn russian_source_text_is_translated_to_english() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        set_language(Some("en"));
        assert_eq!(tr("Запустить игру"), "Launch Game");
    }

    #[test]
    fn catalogues_translate_known_strings() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        set_language(Some("fr"));
        let french = tr("Launch Game");
        assert!(!french.is_empty());
        set_language(Some("ru"));
        assert_eq!(tr("Launch Game"), "Запустить игру");
        set_language(Some("en"));
    }

    #[test]
    fn unknown_language_falls_back_to_english_and_empty_uses_system() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        assert_eq!(set_language(Some("nope")), "en");
        let _ = set_language(Some(""));
        set_language(Some("en"));
    }
}

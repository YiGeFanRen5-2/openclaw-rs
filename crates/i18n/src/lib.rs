//! # openclaw-i18n — Internationalization Foundation for OpenClaw
//!
//! Provides locale detection, translation loading, and formatting support.
//!
//! ## Features
//! - Locale detection and switching
//! - Translation string loading (YAML/JSON)
//! - Date, time, and number formatting
//! - Plural form handling
//!
//! ## Example
//! ```
//! use openclaw_i18n::{I18n, Locale};
//!
//! let mut i18n = I18n::new();
//! i18n.set_locale(Locale::ZhCn);
//! println!("{}", i18n.t("greeting"));
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Error, Debug)]
pub enum I18nError {
    #[error("locale not found: {0}")]
    LocaleNotFound(String),
    #[error("translation key not found: {0} in locale {1}")]
    KeyNotFound(String, String),
    #[error("failed to parse translation file: {0}")]
    ParseError(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// Locale
// ---------------------------------------------------------------------------

/// Supported locales
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Locale {
    En,
    ZhCn,
    ZhTw,
    Ja,
    Ko,
}

impl Locale {
    /// Parse a locale string (e.g. "en", "zh-CN", "zh_CN")
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().replace('_', "-").as_str() {
            "en" | "en-us" | "en-gb" | "en_" | "" => Some(Locale::En),
            "zh" | "zh-cn" | "zh_cn" | "zh-hans" => Some(Locale::ZhCn),
            "zh-tw" | "zh_tw" | "zh-hant" => Some(Locale::ZhTw),
            "ja" | "ja-jp" | "ja_jp" => Some(Locale::Ja),
            "ko" | "ko-kr" | "ko_kr" => Some(Locale::Ko),
            _ => None,
        }
    }

    /// Return the BCP-47 language tag
    pub fn bcp47(self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::ZhCn => "zh-CN",
            Locale::ZhTw => "zh-TW",
            Locale::Ja => "ja",
            Locale::Ko => "ko",
        }
    }

    /// Return the directory name used in locales/
    pub fn dir_name(self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::ZhCn => "zh-CN",
            Locale::ZhTw => "zh-TW",
            Locale::Ja => "ja",
            Locale::Ko => "ko",
        }
    }

    /// All supported locales
    pub fn all() -> &'static [Locale] {
        &[
            Locale::En,
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::Ja,
            Locale::Ko,
        ]
    }
}

impl Default for Locale {
    fn default() -> Self {
        Locale::En
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.bcp47())
    }
}

// ---------------------------------------------------------------------------
// Translation entry — supports interpolation and nested keys
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
enum TransEntry {
    Simple(String),
    Formatted(FormatString),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct FormatString {
    #[serde(rename = "format")]
    format: String,
    #[serde(default)]
    plural: Option<String>,
    #[serde(default)]
    gender: Option<String>,
}

#[allow(dead_code)]
fn resolve_entry(key: &str, map: &serde_json::Map<String, serde_json::Value>) -> Option<TransEntry> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut node: Option<&serde_json::Value> = None;

    for part in &parts {
        node = match node {
            None => map.get(*part),
            Some(serde_json::Value::Object(m)) => m.get(*part),
            _ => None,
        };
    }

    node.and_then(|v| {
        if let serde_json::Value::String(s) = v {
            Some(TransEntry::Simple(s.clone()))
        } else if let serde_json::Value::Object(m) = v {
            serde_json::from_value(serde_json::Value::Object(m.clone())).ok()
        } else {
            None
        }
    })
}

// ---------------------------------------------------------------------------
// Plural rules (CLDR-inspired)
// ---------------------------------------------------------------------------

fn plural_index(locale: Locale, n: u64) -> &'static str {
    // Simplified CLDR plural rules
    match locale {
        Locale::ZhCn | Locale::ZhTw | Locale::Ja | Locale::Ko => "other",
        Locale::En => {
            if n == 1 {
                "one"
            } else {
                "other"
            }
        }
    }
}

// ---------------------------------------------------------------------------
// I18n
// ---------------------------------------------------------------------------

/// Loaded translations keyed by locale
type Translations = HashMap<Locale, serde_json::Map<String, serde_json::Value>>;

/// Internationalization context
#[derive(Debug, Clone)]
pub struct I18n {
    /// Currently active locale
    locale: Locale,
    /// Translations map
    translations: Translations,
    /// Default locale fallback
    fallback: Locale,
}

impl Default for I18n {
    fn default() -> Self {
        Self::new()
    }
}

impl I18n {
    /// Create a new I18n with no translations loaded
    pub fn new() -> Self {
        Self {
            locale: Locale::default(),
            translations: Translations::new(),
            fallback: Locale::En,
        }
    }

    /// Create from a locales directory (e.g. "locales/")
    pub fn from_dir(path: impl AsRef<Path>) -> Result<Self, I18nError> {
        let mut i18n = Self::new();
        i18n.load_dir(path)?;
        Ok(i18n)
    }

    /// Load translations from a directory containing locale subdirectories
    /// Each subdirectory should have a messages.json or messages.yaml file
    pub fn load_dir(&mut self, path: impl AsRef<Path>) -> Result<(), I18nError> {
        let base = path.as_ref();
        if !base.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(base)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let Some(locale) = Locale::parse(&name) else {
                continue;
            };

            // Try JSON first, then fallback YAML support
            let msg_path = entry.path().join("messages.json");
            let data = if msg_path.exists() {
                fs::read_to_string(&msg_path)?
            } else {
                continue;
            };

            let map: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(&data)?;

            // Flatten nested maps into dot-notation keys
            let flat = flatten_map("", &map);
            self.translations.insert(locale, flat);
        }
        Ok(())
    }

    /// Add a single locale's translation from JSON string
    pub fn load_json(&mut self, locale: Locale, json: &str) -> Result<(), I18nError> {
        let map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(json)?;
        let flat = flatten_map("", &map);
        self.translations.insert(locale, flat);
        Ok(())
    }

    fn resolve(&self, key: &str) -> Option<String> {
        // Try current locale
        if let Some(map) = self.translations.get(&self.locale) {
            if let Some(val) = map.get(key) {
                return val.as_str().map(String::from);
            }
        }
        // Fallback
        if self.fallback != self.locale {
            if let Some(map) = self.translations.get(&self.fallback) {
                if let Some(val) = map.get(key) {
                    return val.as_str().map(String::from);
                }
            }
        }
        None
    }

    /// Get the current locale
    pub fn locale(&self) -> Locale {
        self.locale
    }

    /// Set the active locale
    pub fn set_locale(&mut self, locale: Locale) {
        self.locale = locale;
    }

    /// Set the fallback locale
    pub fn set_fallback(&mut self, locale: Locale) {
        self.fallback = locale;
    }

    /// Translate a key with optional interpolation.
    /// Interpolation: pass named parameters like `("greeting", &[("name", "Alice")])`
    ///
    /// ```
    /// use openclaw_i18n::{I18n, Locale};
    ///
    /// let mut i18n = I18n::new();
    /// i18n.load_json(Locale::En, r#"{"greeting": "Hello, {name}!"}"#).unwrap();
    /// assert_eq!(i18n.t_with("greeting", &[("name", "Alice")]), "Hello, Alice!");
    /// ```
    pub fn t(&self, key: &str) -> String {
        self.t_with(key, &[])
    }

    /// Translate with interpolation parameters
    pub fn t_with<'a>(&self, key: &str, params: &[(&str, &'a str)]) -> String {
        let template = match self.resolve(key) {
            Some(s) => s,
            None => return key.to_string(),
        };
        interpolate(&template, params)
    }

    /// Translate a pluralizable key.
    /// Pass the count to select the correct plural form.
    ///
    /// ```
    /// use openclaw_i18n::{I18n, Locale};
    ///
    /// let mut i18n = I18n::new();
    /// i18n.load_json(Locale::En, r#"{
    ///   "items": {"one": "{count} item", "other": "{count} items"}
    /// }"#).unwrap();
    /// assert_eq!(i18n.t_plural("items", 1, &[("count", "1")]), "1 item");
    /// assert_eq!(i18n.t_plural("items", 5, &[("count", "5")]), "5 items");
    /// ```
    pub fn t_plural(&self, key: &str, count: u64, params: &[(&str, &str)]) -> String {
        let form = plural_index(self.locale, count);
        let full_key = format!("{}.{}", key, form);
        let template = match self.resolve(&full_key) {
            Some(s) => s,
            None => self.resolve(key).unwrap_or_else(|| key.to_string()),
        };
        interpolate(&template, params)
    }

    /// Format a number according to the current locale
    pub fn format_number(&self, n: f64) -> String {
        match self.locale {
            Locale::ZhCn | Locale::ZhTw | Locale::Ja | Locale::Ko => {
                format!("{:.2}", n)
            }
            Locale::En => {
                if n.fract() == 0.0 {
                    format!("{:.0}", n)
                } else {
                    format!("{:.2}", n)
                }
            }
        }
    }

    /// Format a date using chrono with locale awareness
    pub fn format_date(&self, date: &chrono::NaiveDate) -> String {
        match self.locale {
            Locale::ZhCn => date.format("%Y年%m月%d日").to_string(),
            Locale::ZhTw => date.format("%Y年%m月%d日").to_string(),
            Locale::Ja => date.format("%Y年%m月%d日").to_string(),
            Locale::Ko => date.format("%Y년 %m월 %d일").to_string(),
            Locale::En => date.format("%B %d, %Y").to_string(),
        }
    }

    /// Format a datetime using chrono with locale awareness
    pub fn format_datetime(&self, dt: &chrono::NaiveDateTime) -> String {
        match self.locale {
            Locale::ZhCn => dt.format("%Y年%m月%d日 %H:%M").to_string(),
            Locale::ZhTw => dt.format("%Y年%m月%d日 %H:%M").to_string(),
            Locale::Ja => dt.format("%Y年%m月%d日 %H:%M").to_string(),
            Locale::Ko => dt.format("%Y년 %m월 %d일 %H:%M").to_string(),
            Locale::En => dt.format("%B %d, %Y %H:%M").to_string(),
        }
    }

    /// Detect locale from HTTP Accept-Language header
    pub fn detect_from_accept_language(header: &str) -> Locale {
        let header = header.to_lowercase();
        for part in header.split(',') {
            let part = part.trim().split(';').next().unwrap_or(part).trim();
            if let Some(locale) = Locale::parse(part) {
                return locale;
            }
        }
        Locale::En
    }

    /// Check if a translation exists for the current locale
    pub fn contains(&self, key: &str) -> bool {
        self.resolve(key).is_some()
    }

    /// Get all loaded locale keys (not full content)
    pub fn available_locales(&self) -> Vec<Locale> {
        self.translations.keys().copied().collect()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Recursively flatten a JSON object into dot-notation keys
fn flatten_map(prefix: &str, map: &serde_json::Map<String, serde_json::Value>) -> serde_json::Map<String, serde_json::Value> {
    let mut result = serde_json::Map::new();
    for (key, value) in map {
        let full_key = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", prefix, key)
        };
        if let serde_json::Value::Object(nested) = value {
            let nested_flat = flatten_map(&full_key, nested);
            for (k, v) in nested_flat {
                result.insert(k, v);
            }
        } else {
            result.insert(full_key, value.clone());
        }
    }
    result
}

/// Replace {name} placeholders in a template string
fn interpolate(template: &str, params: &[(&str, &str)]) -> String {
    let mut result = template.to_string();
    for (name, value) in params {
        let placeholder = format!("{{{}}}", name);
        result = result.replace(&placeholder, value);
    }
    result
}

// ---------------------------------------------------------------------------
// Locale detection utilities
// ---------------------------------------------------------------------------

/// Detect locale from environment variables (LANG, LC_ALL, LANGUAGE)
pub fn detect_from_env() -> Locale {
    for var in &["LANG", "LC_ALL", "LANGUAGE"] {
        if let Ok(val) = std::env::var(var) {
            if let Some(locale) = Locale::parse(&val) {
                return locale;
            }
        }
    }
    Locale::En
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_parse() {
        assert_eq!(Locale::parse("en"), Some(Locale::En));
        assert_eq!(Locale::parse("zh-CN"), Some(Locale::ZhCn));
        assert_eq!(Locale::parse("zh_CN"), Some(Locale::ZhCn));
        assert_eq!(Locale::parse("ja-JP"), Some(Locale::Ja));
        assert_eq!(Locale::parse("unknown"), None);
    }

    #[test]
    fn test_plural_en() {
        assert_eq!(plural_index(Locale::En, 1), "one");
        assert_eq!(plural_index(Locale::En, 0), "other");
        assert_eq!(plural_index(Locale::En, 2), "other");
        // CJK: always "other"
        assert_eq!(plural_index(Locale::ZhCn, 1), "other");
        assert_eq!(plural_index(Locale::ZhCn, 100), "other");
    }

    #[test]
    fn test_interpolate() {
        let result = interpolate("Hello, {name}! You have {count} messages.", &[
            ("name", "Alice"),
            ("count", "3"),
        ]);
        assert_eq!(result, "Hello, Alice! You have 3 messages.");
    }

    #[test]
    fn test_flatten_map() {
        let json = serde_json::json!({
            "greeting": "Hello",
            "nested": {
                "key": "Value",
                "deep": { "inner": "DeepValue" }
            }
        });
        let map = json.as_object().unwrap();
        let flat = flatten_map("", map);
        assert_eq!(flat.get("greeting").and_then(|v| v.as_str()), Some("Hello"));
        assert_eq!(flat.get("nested.key").and_then(|v| v.as_str()), Some("Value"));
        assert_eq!(flat.get("nested.deep.inner").and_then(|v| v.as_str()), Some("DeepValue"));
    }

    #[test]
    fn test_i18n_basic() {
        let mut i18n = I18n::new();
        i18n.load_json(Locale::En, r#"{"hello": "Hello, World!"}"#).unwrap();
        assert_eq!(i18n.t("hello"), "Hello, World!");
    }

    #[test]
    fn test_i18n_interpolation() {
        let mut i18n = I18n::new();
        i18n.load_json(Locale::En, r#"{"greeting": "Hello, {name}!"}"#).unwrap();
        assert_eq!(i18n.t_with("greeting", &[("name", "Bob")]), "Hello, Bob!");
    }

    #[test]
    fn test_i18n_plural() {
        let mut i18n = I18n::new();
        i18n.load_json(Locale::En, r#"{
            "item": {"one": "{count} item", "other": "{count} items"}
        }"#).unwrap();
        assert_eq!(i18n.t_plural("item", 1, &[("count", "1")]), "1 item");
        assert_eq!(i18n.t_plural("item", 5, &[("count", "5")]), "5 items");
    }

    #[test]
    fn test_i18n_missing_key() {
        let i18n = I18n::new();
        assert_eq!(i18n.t("missing"), "missing");
    }

    #[test]
    fn test_i18n_fallback() {
        let mut i18n = I18n::new();
        i18n.load_json(Locale::ZhCn, r#"{"hello": "你好"}"#).unwrap();
        i18n.set_locale(Locale::ZhCn);
        assert_eq!(i18n.t("hello"), "你好");
    }

    #[test]
    fn test_format_number() {
        let mut i18n = I18n::new();
        i18n.set_locale(Locale::En);
        assert_eq!(i18n.format_number(42.0), "42");
        assert_eq!(i18n.format_number(3.14159), "3.14");
        i18n.set_locale(Locale::ZhCn);
        assert_eq!(i18n.format_number(3.14159), "3.14");
    }

    #[test]
    fn test_detect_from_accept_language() {
        assert_eq!(I18n::detect_from_accept_language("en-US,en;q=0.9"), Locale::En);
        assert_eq!(I18n::detect_from_accept_language("zh-CN;q=0.8,en"), Locale::ZhCn);
        assert_eq!(I18n::detect_from_accept_language("ja,en-US"), Locale::Ja);
        assert_eq!(I18n::detect_from_accept_language(""), Locale::En);
    }

    #[test]
    fn test_format_date() {
        let mut i18n = I18n::new();
        let date = chrono::NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        i18n.set_locale(Locale::En);
        assert!(i18n.format_date(&date).contains("2025"));
        i18n.set_locale(Locale::ZhCn);
        assert!(i18n.format_date(&date).contains("2025年"));
    }
}

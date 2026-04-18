//! # Example 11: Internationalization (i18n)
//!
//! Demonstrates the openclaw-i18n crate: locale detection, translation loading,
//! interpolation, plural handling, and date/number formatting.
//!
//! Run with: cargo run --example example_11_i18n

use openclaw_i18n::{detect_from_env, I18n, Locale};
use chrono::{NaiveDate, NaiveDateTime};

fn main() {
    println!("=== OpenClaw i18n Demo ===\n");

    // 1. Build I18n from the bundled locales directory
    let locales_path = concat!(env!("CARGO_MANIFEST_DIR"), "/crates/i18n/locales");
    let mut i18n = I18n::from_dir(locales_path).expect("failed to load locales");

    // 2. Detect locale from environment
    let detected = detect_from_env();
    println!("Detected locale from environment: {}", detected);

    // 3. Basic translation
    println!("\n--- Basic Translation ---");
    i18n.set_locale(Locale::En);
    println!("EN | app.name: {}", i18n.t("app.name"));
    println!("EN | common.ok: {}", i18n.t("common.ok"));

    i18n.set_locale(Locale::ZhCn);
    println!("ZH | app.name: {}", i18n.t("app.name"));
    println!("ZH | common.ok: {}", i18n.t("common.ok"));

    // 4. Interpolation
    println!("\n--- Interpolation ---");
    i18n.set_locale(Locale::En);
    println!("EN | errors.timeout: {}", i18n.t_with("errors.timeout", &[]));
    println!("EN | time.minutes_ago: {}", i18n.t_with("time.minutes_ago", &[("count", "5")]));
    println!("EN | files.uploaded: {}", i18n.t_with("files.uploaded", &[("name", "report.pdf")]));

    i18n.set_locale(Locale::ZhCn);
    println!("ZH | files.uploaded: {}", i18n.t_with("files.uploaded", &[("name", "报告.pdf")]));

    // 5. Plural forms
    println!("\n--- Plural Forms ---");
    i18n.set_locale(Locale::En);
    println!("EN | files.item (1): {}", i18n.t_plural("files.item", 1, &[("count", "1")]));
    println!("EN | files.item (42): {}", i18n.t_plural("files.item", 42, &[("count", "42")]));
    println!("EN | files.folder (1): {}", i18n.t_plural("files.folder", 1, &[("count", "1")]));
    println!("EN | files.folder (7): {}", i18n.t_plural("files.folder", 7, &[("count", "7")]));

    i18n.set_locale(Locale::ZhCn);
    println!("ZH | files.item (5): {}", i18n.t_plural("files.item", 5, &[("count", "5")]));

    // 6. Number formatting
    println!("\n--- Number Formatting ---");
    i18n.set_locale(Locale::En);
    println!("EN | format_number(3.14159): {}", i18n.format_number(3.14159));
    println!("EN | format_number(42.0): {}", i18n.format_number(42.0));

    i18n.set_locale(Locale::ZhCn);
    println!("ZH | format_number(3.14159): {}", i18n.format_number(3.14159));
    println!("ZH | format_number(9999.99): {}", i18n.format_number(9999.99));

    // 7. Date formatting
    println!("\n--- Date Formatting ---");
    let date = NaiveDate::from_ymd_opt(2025, 6, 18).unwrap();
    let datetime = NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap());

    i18n.set_locale(Locale::En);
    println!("EN | format_date: {}", i18n.format_date(&date));
    println!("EN | format_datetime: {}", i18n.format_datetime(&datetime));

    i18n.set_locale(Locale::ZhCn);
    println!("ZH | format_date: {}", i18n.format_date(&date));
    println!("ZH | format_datetime: {}", i18n.format_datetime(&datetime));

    // 8. Accept-Language header parsing
    println!("\n--- Accept-Language Detection ---");
    let tests = &[
        "en-US,en;q=0.9",
        "zh-CN,zh;q=0.8,en;q=0.5",
        "ja,en-US;q=0.7",
        "ko-KR",
        "",
    ];
    for header in tests {
        println!("  {:?} → {}", header, I18n::detect_from_accept_language(header));
    }

    // 9. Available locales
    println!("\n--- Available Locales ---");
    println!("  Loaded: {:?}", i18n.available_locales());
    println!("  Current: {}", i18n.locale());

    // 10. Missing key fallback
    println!("\n--- Missing Key Fallback ---");
    i18n.set_locale(Locale::En);
    println!("  Missing key returns raw key: '{}'", i18n.t("completely.missing.key"));

    // 11. All supported locales
    println!("\n--- All Supported Locales ---");
    for &locale in Locale::all() {
        println!("  {} ({})", locale, locale.bcp47());
    }

    println!("\n=== Demo complete ===");
}

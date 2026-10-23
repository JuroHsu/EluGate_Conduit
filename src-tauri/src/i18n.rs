//! 多語系：與前端共用專案根目錄的 `locales/*.json`（扁平鍵值，`{name}` 為參數）。

use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

/// 新增語系時在此加入代碼（需有對應的 locales/<代碼>.json），並在 `normalize` 對應系統語系
macro_rules! locales {
    ($($lang:literal),+ $(,)?) => {
        /// 支援的語系；第一個為後備語系
        pub const LANGUAGES: &[&str] = &[$($lang),+];
        const RAW: &[(&str, &str)] = &[$(($lang, include_str!(concat!("../../locales/", $lang, ".json")))),+];
    };
}

locales!("en", "zh-TW", "zh-CN", "ja", "ko", "es", "fr", "de", "pt-BR", "ru", "vi");

static BUNDLES: LazyLock<HashMap<&'static str, HashMap<String, String>>> = LazyLock::new(|| {
    RAW.iter()
        .map(|&(lang, text)| (lang, serde_json::from_str(text).unwrap_or_else(|e| panic!("locales/{lang}.json: {e}"))))
        .collect()
});

static CURRENT: RwLock<&'static str> = RwLock::new("en");

/// 將系統或使用者指定的語系代碼對應到支援的語系
pub fn normalize(tag: &str) -> &'static str {
    // POSIX locales may carry an encoding/modifier, e.g. zh_TW.UTF-8.
    let t = tag.trim().split(['.', '@']).next().unwrap_or("").to_ascii_lowercase().replace('_', "-");
    if t.starts_with("zh") {
        if t.contains("hant") || t.ends_with("-tw") || t.ends_with("-hk") || t.ends_with("-mo") {
            "zh-TW"
        } else {
            "zh-CN"
        }
    } else {
        let base = t.split('-').next().unwrap_or("");
        match base {
            "ja" => "ja",
            "ko" => "ko",
            "es" => "es",
            "fr" => "fr",
            "de" => "de",
            "pt" => "pt-BR",
            "ru" => "ru",
            "vi" => "vi",
            _ => "en",
        }
    }
}

/// `pref` 為空或 "auto" 時跟隨系統語系
pub fn set_language(pref: Option<&str>) -> &'static str {
    let lang = match pref.map(str::trim).filter(|p| !p.is_empty() && *p != "auto") {
        Some(p) => normalize(p),
        None => normalize(&sys_locale::get_locale().unwrap_or_default()),
    };
    *CURRENT.write().unwrap() = lang;
    lang
}

pub fn current() -> &'static str {
    *CURRENT.read().unwrap()
}

pub fn t(key: &str, args: &[(&str, String)]) -> String {
    let lang = current();
    let template = BUNDLES
        .get(lang)
        .and_then(|b| b.get(key))
        .or_else(|| BUNDLES.get("en").and_then(|b| b.get(key)))
        .map(String::as_str)
        .unwrap_or(key);
    let mut out = template.to_string();
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

/// `tr!("key")` 或 `tr!("key", name = value, ...)`
#[macro_export]
macro_rules! tr {
    ($key:expr) => {
        $crate::i18n::t($key, &[])
    };
    ($key:expr, $($name:ident = $val:expr),+ $(,)?) => {
        $crate::i18n::t($key, &[$((stringify!($name), ($val).to_string())),+])
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_locales_have_same_keys() {
        let en: std::collections::BTreeSet<_> = BUNDLES["en"].keys().collect();
        for lang in LANGUAGES {
            let keys: std::collections::BTreeSet<_> = BUNDLES[lang].keys().collect();
            let missing: Vec<_> = en.difference(&keys).collect();
            let extra: Vec<_> = keys.difference(&en).collect();
            assert!(missing.is_empty() && extra.is_empty(), "{lang}: missing {missing:?}, extra {extra:?}");
        }
    }

    #[test]
    fn normalizes_tags() {
        assert_eq!(normalize("zh-Hant-TW"), "zh-TW");
        assert_eq!(normalize("zh_HK"), "zh-TW");
        assert_eq!(normalize("zh_TW.UTF-8"), "zh-TW");
        assert_eq!(normalize("zh_HK.UTF-8@variant"), "zh-TW");
        assert_eq!(normalize("zh-MO"), "zh-TW");
        assert_eq!(normalize("zh-CN"), "zh-CN");
        assert_eq!(normalize("ja-JP"), "ja");
        assert_eq!(normalize("ko-KR"), "ko");
        assert_eq!(normalize("fr-FR"), "fr");
        assert_eq!(normalize("pt-PT"), "pt-BR");
        assert_eq!(normalize("de_DE"), "de");
        assert_eq!(normalize("it-IT"), "en");
    }
}

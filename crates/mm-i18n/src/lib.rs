use std::collections::HashMap;

/// Internationalization support using fluent-style messages.
pub struct I18n {
    locale: String,
    messages: HashMap<String, String>,
}

impl I18n {
    pub fn new(locale: &str) -> Self {
        let messages = match locale {
            "ja" => load_japanese(),
            _ => load_english(),
        };
        Self {
            locale: locale.to_string(),
            messages,
        }
    }

    pub fn locale(&self) -> &str {
        &self.locale
    }

    /// Get a translated message by key.
    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.messages.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    /// Get a translated message with interpolation.
    pub fn format(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut text = self.get(key).to_string();
        for (name, value) in args {
            text = text.replace(&format!("{{{name}}}"), value);
        }
        text
    }

    /// Switch locale at runtime.
    pub fn set_locale(&mut self, locale: &str) {
        self.locale = locale.to_string();
        self.messages = match locale {
            "ja" => load_japanese(),
            _ => load_english(),
        };
    }

    /// All known message keys.
    pub fn keys(&self) -> Vec<&String> {
        self.messages.keys().collect()
    }
}

fn load_english() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("app.title".into(), "MangaMeeya".into());
    m.insert("menu.file".into(), "File".into());
    m.insert("menu.open".into(), "Open".into());
    m.insert("menu.save".into(), "Save".into());
    m.insert("menu.quit".into(), "Quit".into());
    m.insert("menu.view".into(), "View".into());
    m.insert("menu.fullscreen".into(), "Fullscreen".into());
    m.insert("menu.settings".into(), "Settings".into());
    m.insert("page.info".into(), "Page {current}/{total}".into());
    m.insert("page.next".into(), "Next Page".into());
    m.insert("page.prev".into(), "Previous Page".into());
    m.insert("page.first".into(), "First Page".into());
    m.insert("page.last".into(), "Last Page".into());
    m.insert("mode.single".into(), "Single Page".into());
    m.insert("mode.dual".into(), "Dual Page".into());
    m.insert("error.open_failed".into(), "Failed to open: {path}".into());
    m.insert("error.decode_failed".into(), "Failed to decode image".into());
    m
}

fn load_japanese() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("app.title".into(), "MangaMeeya".into());
    m.insert("menu.file".into(), "ファイル".into());
    m.insert("menu.open".into(), "開く".into());
    m.insert("menu.save".into(), "保存".into());
    m.insert("menu.quit".into(), "終了".into());
    m.insert("menu.view".into(), "表示".into());
    m.insert("menu.fullscreen".into(), "全画面".into());
    m.insert("menu.settings".into(), "設定".into());
    m.insert("page.info".into(), "ページ {current}/{total}".into());
    m.insert("page.next".into(), "次のページ".into());
    m.insert("page.prev".into(), "前のページ".into());
    m.insert("page.first".into(), "最初のページ".into());
    m.insert("page.last".into(), "最後のページ".into());
    m.insert("mode.single".into(), "単一ページ".into());
    m.insert("mode.dual".into(), "見開きページ".into());
    m.insert("error.open_failed".into(), "開けませんでした: {path}".into());
    m.insert("error.decode_failed".into(), "画像のデコードに失敗しました".into());
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_english() {
        let i18n = I18n::new("en");
        assert_eq!(i18n.locale(), "en");
        assert_eq!(i18n.get("menu.file"), "File");
    }

    #[test]
    fn test_load_japanese() {
        let i18n = I18n::new("ja");
        assert_eq!(i18n.locale(), "ja");
        assert_eq!(i18n.get("menu.file"), "ファイル");
    }

    #[test]
    fn test_missing_key() {
        let i18n = I18n::new("en");
        assert_eq!(i18n.get("nonexistent.key"), "nonexistent.key");
    }

    #[test]
    fn test_format_interpolation() {
        let i18n = I18n::new("en");
        let result = i18n.format("page.info", &[("current", "5"), ("total", "100")]);
        assert_eq!(result, "Page 5/100");
    }

    #[test]
    fn test_format_interpolation_japanese() {
        let i18n = I18n::new("ja");
        let result = i18n.format("page.info", &[("current", "5"), ("total", "100")]);
        assert_eq!(result, "ページ 5/100");
    }

    #[test]
    fn test_switch_locale() {
        let mut i18n = I18n::new("en");
        assert_eq!(i18n.get("menu.file"), "File");
        i18n.set_locale("ja");
        assert_eq!(i18n.get("menu.file"), "ファイル");
    }

    #[test]
    fn test_all_keys_present_both_locales() {
        let en = I18n::new("en");
        let ja = I18n::new("ja");
        let en_keys: std::collections::HashSet<_> = en.keys().into_iter().collect();
        let ja_keys: std::collections::HashSet<_> = ja.keys().into_iter().collect();
        assert_eq!(en_keys, ja_keys);
    }
}

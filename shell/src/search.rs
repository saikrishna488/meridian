//! Indexed settings and bounded arithmetic search. No shell or JavaScript evaluation.
use meridian_protocol::{PreferencesPage as Page, ProviderId, SearchItem, SearchPreferences, SearchSection};
use meridian_services::search::fuzzy::{Prepared, fold_token, fuzzy};

pub const SETTINGS: &[(&str, Page, &str, &str)] = &[
    ("wifi", Page::Wifi, "Wi-Fi", "wireless internet network connection"),
    ("bluetooth", Page::Bluetooth, "Bluetooth", "pair headphones keyboard mouse"),
    ("displays", Page::Displays, "Displays", "monitor screen resolution scale refresh brightness primary"),
    ("sound", Page::Sound, "Sound", "audio speaker microphone volume input output mute"),
    ("power", Page::Power, "Power & Lock", "battery suspend sleep security idle lock charging"),
    ("appearance", Page::Appearance, "Appearance", "dark font typography"),
    ("wallpaper", Page::Wallpaper, "Wallpaper", "background image desktop"),
    ("startup-apps", Page::StartupApps, "Startup Apps", "autostart login applications"),
    ("applications", Page::Applications, "Applications", "installed apps software programs launch uninstall"),
    ("general", Page::General, "General", "about updates kernel version memory processor"),
    ("users", Page::Users, "Users", "account password administrator"),
    ("storage", Page::Storage, "Storage", "disk volume filesystem mount usb removable capacity"),
    ("search", Page::Search, "Search", "launcher providers calculator privacy"),
];
pub fn page(id: &str) -> Option<Page> {
    SETTINGS.iter().find(|entry| entry.0 == id).map(|entry| entry.1.clone())
}
pub fn preferences() -> SearchPreferences {
    crate::config::read_versioned(&gtk::glib::user_config_dir().join("meridian/search.json")).unwrap_or_default()
}
pub fn save(value: &SearchPreferences) -> Result<(), meridian_protocol::ErrorBody> {
    crate::config::save_versioned(&gtk::glib::user_config_dir().join("meridian/search.json"), value)
}
pub fn sections(query: &str, preferences: &SearchPreferences) -> Vec<SearchSection> {
    let mut sections = vec![];
    if query.trim().is_empty() {
        return sections;
    }
    if preferences.settings {
        let tokens: Vec<_> = query.split_whitespace().map(fold_token).collect();
        let mut ranked: Vec<_> = SETTINGS
            .iter()
            .filter_map(|(id, _, title, keywords)| {
                let name = Prepared::new(title);
                let words = Prepared::new(keywords);
                let score: Option<i32> = tokens
                    .iter()
                    .map(|token| {
                        fuzzy(token, &name).map(|score| score * 2).into_iter().chain(fuzzy(token, &words)).max()
                    })
                    .sum();
                score.map(|score| {
                    (
                        score,
                        SearchItem {
                            id: (*id).into(),
                            title: (*title).into(),
                            subtitle: Some("System settings".into()),
                            icon: Some("preferences-system".into()),
                        },
                    )
                })
            })
            .collect();
        ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.title.cmp(&b.1.title)));
        if !ranked.is_empty() {
            sections.push(SearchSection {
                provider: ProviderId::Settings,
                title: "Settings".into(),
                items: ranked.into_iter().take(8).map(|(_, item)| item).collect(),
            });
        }
    }
    if preferences.calculator
        && let Some(value) = calculate(query)
    {
        sections.push(SearchSection {
            provider: ProviderId::Calculator,
            title: "Calculator".into(),
            items: vec![SearchItem {
                id: query.into(),
                title: value,
                subtitle: Some("Copy result".into()),
                icon: Some("accessories-calculator".into()),
            }],
        });
    }
    sections
}
pub fn calculate(text: &str) -> Option<String> {
    if text.len() > 256 || !text.bytes().any(|c| b"+-*/%".contains(&c)) {
        return None;
    }
    let mut parser = Parser { text: text.as_bytes(), at: 0 };
    let value = parser.expression(0)?;
    parser.space();
    if parser.at != parser.text.len() || !value.is_finite() {
        return None;
    }
    let value = if value == 0.0 { 0.0 } else { value };
    Some(if value.abs() >= 1e12 || (value != 0.0 && value.abs() < 1e-8) {
        format!("{value:.10e}")
    } else {
        format!("{value:.10}").trim_end_matches('0').trim_end_matches('.').to_owned()
    })
}
struct Parser<'a> {
    text: &'a [u8],
    at: usize,
}
impl Parser<'_> {
    fn space(&mut self) {
        while self.text.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }
    fn eat(&mut self, value: u8) -> bool {
        self.space();
        if self.text.get(self.at) == Some(&value) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn expression(&mut self, depth: usize) -> Option<f64> {
        let mut value = self.term(depth)?;
        loop {
            if self.eat(b'+') {
                value += self.term(depth)?;
            } else if self.eat(b'-') {
                value -= self.term(depth)?;
            } else {
                return value.is_finite().then_some(value);
            }
        }
    }
    fn term(&mut self, depth: usize) -> Option<f64> {
        let mut value = self.atom(depth)?;
        loop {
            if self.eat(b'*') {
                value *= self.atom(depth)?;
            } else if self.eat(b'/') {
                value /= self.atom(depth)?;
            } else if self.eat(b'%') {
                value %= self.atom(depth)?;
            } else {
                return value.is_finite().then_some(value);
            }
        }
    }
    fn atom(&mut self, depth: usize) -> Option<f64> {
        if depth >= 32 {
            return None;
        }
        if self.eat(b'+') {
            return self.atom(depth + 1);
        }
        if self.eat(b'-') {
            return Some(-self.atom(depth + 1)?);
        }
        if self.eat(b'(') {
            let value = self.expression(depth + 1)?;
            return self.eat(b')').then_some(value);
        }
        self.space();
        let start = self.at;
        while self.text.get(self.at).is_some_and(|c| c.is_ascii_digit() || *c == b'.') {
            self.at += 1;
        }
        if self.at == start {
            return None;
        }
        if self.text.get(self.at).is_some_and(|c| *c == b'e' || *c == b'E') {
            self.at += 1;
            if self.text.get(self.at).is_some_and(|c| *c == b'+' || *c == b'-') {
                self.at += 1;
            }
            while self.text.get(self.at).is_some_and(u8::is_ascii_digit) {
                self.at += 1;
            }
        }
        let value = std::str::from_utf8(&self.text[start..self.at]).ok()?.parse::<f64>().ok()?;
        value.is_finite().then_some(value)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arithmetic_is_bounded_and_respects_precedence() {
        for (text, expected) in
            [("2+3*4", "14"), ("(2+3)*4", "20"), ("-2 * -3", "6"), ("0.1+0.2", "0.3"), ("10%3", "1"), ("1e2/4", "25")]
        {
            assert_eq!(calculate(text).as_deref(), Some(expected));
        }
        for text in ["1/0", "2+alert(1)", "1+NaN", "1+2 garbage", "1e999+2", "1..2+3", "1+(2*3", "firefox", "42"] {
            assert_eq!(calculate(text), None, "{text}");
        }
        assert!(calculate(&format!("{}1+1{}", "(".repeat(40), ")".repeat(40))).is_none());
        assert!(calculate(&"1+".repeat(200)).is_none());
    }
    #[test]
    fn settings_are_allowlisted_and_providers_can_be_disabled() {
        assert_eq!(page("displays"), Some(Page::Displays));
        assert!(page("../../shell").is_none());
        assert!(sections("wifi", &SearchPreferences::default()).iter().any(|s| s.items.iter().any(|i| i.id == "wifi")));
        assert!(sections("wifi", &SearchPreferences { settings: false, calculator: false }).is_empty());
        assert!(sections("", &SearchPreferences { settings: true, calculator: true }).is_empty());
    }
}

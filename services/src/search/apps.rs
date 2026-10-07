//! Application search provider, backed by the [`AppCatalog`].

use std::cell::RefCell;
use std::rc::Rc;

use meridian_protocol::{ErrorBody, ErrorCode, ProviderId, SearchItem};

use super::fuzzy::{self, Prepared};
use super::{ActivationContext, Query, SearchProvider};
use crate::apps::{App, AppCatalog};

/// Field weights, in percent.
const NAME_WEIGHT: i32 = 100;
const ALIAS_WEIGHT: i32 = 70; // generic name, keywords
const DESCRIPTION_WEIGHT: i32 = 40;

pub struct AppsProvider {
    catalog: Rc<RefCell<AppCatalog>>,
}

impl AppsProvider {
    pub fn new(catalog: Rc<RefCell<AppCatalog>>) -> Self {
        AppsProvider { catalog }
    }

    fn item(&self, app: &App) -> SearchItem {
        SearchItem {
            id: app.id.clone(),
            title: app.name.clone(),
            icon: app.icon.clone(),
            subtitle: app.generic_name.clone().or_else(|| app.description.clone()),
        }
    }
}

/// Score of an app for a non-empty query: every token must match some field.
pub fn score_app(app: &App, query: &Query) -> Option<i32> {
    let f = &app.search;
    let mut total = 0;
    for token in &query.tokens {
        let best = [
            weighted(fuzzy::fuzzy(token, &f.name), NAME_WEIGHT),
            f.aliases.iter().filter_map(|a| weighted(fuzzy::fuzzy(token, a), ALIAS_WEIGHT)).max(),
            weighted(fuzzy::word_prefix(token, &f.description), DESCRIPTION_WEIGHT),
        ]
        .into_iter()
        .flatten()
        .max()?;
        total += best;
    }
    Some(total)
}

fn weighted(score: Option<i32>, weight: i32) -> Option<i32> {
    score.map(|s| s * weight / 100)
}

/// Search fields of an app, prepared once when the catalog loads.
#[derive(Debug, Clone, Default)]
pub struct SearchFields {
    pub name: Prepared,
    pub aliases: Vec<Prepared>,
    pub description: Prepared,
}

impl SearchFields {
    pub fn new(name: &str, generic_name: Option<&str>, keywords: &[String], description: Option<&str>) -> Self {
        let aliases = generic_name
            .into_iter()
            .chain(keywords.iter().map(String::as_str))
            .map(Prepared::new)
            .filter(|p| !p.is_empty())
            .collect();
        SearchFields { name: Prepared::new(name), aliases, description: Prepared::new(description.unwrap_or("")) }
    }
}

impl SearchProvider for AppsProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Apps
    }

    fn title(&self) -> &str {
        "Applications"
    }

    fn query(&self, query: &Query) -> Vec<SearchItem> {
        let catalog = self.catalog.borrow();
        if query.is_empty() {
            // The catalog is kept sorted by name: this is the home grid.
            return catalog.apps().iter().map(|a| self.item(a)).collect();
        }
        let mut scored: Vec<(i32, &App)> =
            catalog.apps().iter().filter_map(|a| score_app(a, query).map(|s| (s, a))).collect();
        // Stable sort keeps alphabetical order among equal scores.
        scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
        scored.into_iter().map(|(_, a)| self.item(a)).collect()
    }

    fn activate(&self, item_id: &str, ctx: &ActivationContext) -> Result<(), ErrorBody> {
        self.catalog.borrow().launch(item_id, ctx.launch_context).map_err(|e| match e {
            crate::apps::LaunchError::NotFound => ErrorBody::new(ErrorCode::NotFound, "no such application"),
            crate::apps::LaunchError::Failed(msg) => ErrorBody::new(ErrorCode::Failed, msg),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str, generic: Option<&str>, keywords: &[&str], description: Option<&str>) -> App {
        let keywords: Vec<String> = keywords.iter().map(|s| s.to_string()).collect();
        App::for_test(name, generic, &keywords, description)
    }

    fn rank<'a>(apps: &'a [App], q: &str) -> Vec<&'a str> {
        let q = Query::new(q);
        let mut v: Vec<(i32, &App)> = apps.iter().filter_map(|a| score_app(a, &q).map(|s| (s, a))).collect();
        v.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
        v.into_iter().map(|(_, a)| a.name.as_str()).collect()
    }

    fn sample() -> Vec<App> {
        vec![
            app("Firefox", Some("Web Browser"), &["Internet", "WWW"], Some("Browse the World Wide Web")),
            app("Files", Some("File Manager"), &["folder", "explorer"], Some("Access and organize files")),
            app("foot", Some("Terminal"), &["shell", "prompt", "command"], Some("A wayland terminal emulator")),
            app("Text Editor", None, &["notepad", "txt"], Some("Edit text files")),
            app("Settings", None, &["preferences", "control"], Some("Configure the system")),
        ]
    }

    #[test]
    fn name_match_ranks_first() {
        assert_eq!(rank(&sample(), "fire")[0], "Firefox");
        assert_eq!(rank(&sample(), "fil")[0], "Files");
    }

    #[test]
    fn matches_keywords_and_generic_name() {
        assert_eq!(rank(&sample(), "browser")[0], "Firefox");
        assert_eq!(rank(&sample(), "terminal")[0], "foot");
        assert_eq!(rank(&sample(), "notepad"), vec!["Text Editor"]);
    }

    #[test]
    fn matches_description_by_word_prefix_only() {
        assert_eq!(rank(&sample(), "configure"), vec!["Settings"]);
        // "onfig" is inside a word of the description: no description match.
        assert!(!rank(&sample(), "onfigu").contains(&"Settings"));
    }

    #[test]
    fn all_tokens_must_match() {
        assert_eq!(rank(&sample(), "text edit"), vec!["Text Editor"]);
        assert!(rank(&sample(), "firefox terminal").is_empty());
    }

    #[test]
    fn fuzzy_name() {
        assert_eq!(rank(&sample(), "ffx")[0], "Firefox");
    }
}

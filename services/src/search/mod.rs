//! Provider-based search.
//!
//! The UI sends a query string; every registered provider returns a section
//! of results. Activation sends an opaque item id back to the provider that
//! produced it, so providers never execute anything supplied by the UI.

pub mod apps;
pub mod fuzzy;

use meridian_protocol::{ErrorBody, ProviderId, SearchItem, SearchResults, SearchSection};

/// A parsed query, shared by all providers.
#[derive(Debug, Clone)]
pub struct Query {
    pub raw: String,
    /// Whitespace-separated, case-folded tokens. Empty for an empty query.
    pub tokens: Vec<Vec<char>>,
}

impl Query {
    pub fn new(raw: &str) -> Self {
        Query { raw: raw.to_owned(), tokens: raw.split_whitespace().map(fuzzy::fold_token).collect() }
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
}

/// Environment passed to providers when a result is activated.
pub struct ActivationContext<'a> {
    /// Launch context for startup notification / xdg-activation tokens.
    pub launch_context: Option<&'a gio::AppLaunchContext>,
}

pub trait SearchProvider {
    fn id(&self) -> ProviderId;
    fn title(&self) -> &str;
    /// Results for `query`, best first. Must be fast: this runs on every
    /// keystroke on the UI thread. Slow providers must answer from an index.
    fn query(&self, query: &Query) -> Vec<SearchItem>;
    fn activate(&self, item_id: &str, ctx: &ActivationContext) -> Result<(), ErrorBody>;
}

#[derive(Default)]
pub struct SearchService {
    providers: Vec<Box<dyn SearchProvider>>,
}

impl SearchService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, provider: Box<dyn SearchProvider>) {
        assert!(
            self.providers.iter().all(|p| p.id() != provider.id()),
            "search provider {:?} registered twice",
            provider.id()
        );
        self.providers.push(provider);
    }

    pub fn query(&self, raw: &str, serial: u32) -> SearchResults {
        let query = Query::new(raw);
        let sections = self
            .providers
            .iter()
            .map(|p| SearchSection { provider: p.id(), title: p.title().to_owned(), items: p.query(&query) })
            .filter(|s| !s.items.is_empty())
            .collect();
        SearchResults { serial, sections }
    }

    pub fn activate(&self, provider: ProviderId, item_id: &str, ctx: &ActivationContext) -> Result<(), ErrorBody> {
        let p = self
            .providers
            .iter()
            .find(|p| p.id() == provider)
            .ok_or_else(|| ErrorBody::new(meridian_protocol::ErrorCode::NotFound, "unknown provider"))?;
        p.activate(item_id, ctx)
    }
}

#![forbid(unsafe_code)]

//! In-memory navigation state for the local application increment.
//! No profiles, remote URLs, origin authority, HTML, displays or OS handles live here.
//! Limits are development resource bounds, not a claim of complete browser parity.

use std::{fmt, path::PathBuf};

pub const MAX_TABS: usize = 32;
pub const MAX_HISTORY: usize = 128;
pub const MAX_ADDRESS_BYTES: usize = 4096;
const MAX_FAILURE_BYTES: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageSource {
    Welcome,
    Guide,
    Local(PathBuf),
}

impl PageSource {
    pub fn from_address(address: &str) -> Result<Self, NavigationError> {
        // Preserve local filename spelling; stripping spaces could select another
        // file. Built-in addresses alone accept ordinary surrounding spaces.
        // Validate the raw bound/controls before any normalization.
        validate_address_text(address)?;
        match address.trim_matches(' ') {
            "about:welcome" => Ok(Self::Welcome),
            "about:guide" => Ok(Self::Guide),
            _ => {
                crate::input::check_local_path(address).map_err(NavigationError::InvalidAddress)?;
                Ok(Self::Local(PathBuf::from(address)))
            }
        }
    }

    pub fn address(&self) -> String {
        match self {
            Self::Welcome => "about:welcome".to_owned(),
            Self::Guide => "about:guide".to_owned(),
            Self::Local(path) => path.to_string_lossy().into_owned(),
        }
    }

    pub fn title(&self) -> String {
        match self {
            Self::Welcome => "New tab".to_owned(),
            Self::Guide => "Guide".to_owned(),
            Self::Local(path) => path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Local document".to_owned()),
        }
    }

    fn validate(&self) -> Result<(), NavigationError> {
        if let Self::Local(path) = self {
            let address = path.to_str().ok_or_else(|| {
                NavigationError::InvalidAddress("Local filenames must be Unicode".to_owned())
            })?;
            validate_address_text(address)?;
            crate::input::check_local_path(address).map_err(NavigationError::InvalidAddress)?;
        }
        Ok(())
    }
}

fn validate_address_text(address: &str) -> Result<(), NavigationError> {
    if address.trim_matches(' ').is_empty() || address.len() > MAX_ADDRESS_BYTES {
        return Err(NavigationError::InvalidAddress(
            "Address must contain 1..4096 UTF-8 bytes".to_owned(),
        ));
    }
    if address.chars().any(|character| {
        character.is_control()
            || matches!(
                character,
                '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
            )
    }) {
        return Err(NavigationError::InvalidAddress(
            "Control characters are not allowed in addresses".to_owned(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TabId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestKey {
    pub tab: TabId,
    pub generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadReason {
    Navigation,
    Reload,
    Reflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadRequest {
    pub key: RequestKey,
    pub source: PageSource,
    pub reason: LoadReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadStatus {
    Loading,
    Ready,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationError {
    InvalidAddress(String),
    TabLimit,
    HistoryLimit,
    UnknownTab,
    NoHistory,
    CounterExhausted,
}

impl fmt::Display for NavigationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAddress(message) => formatter.write_str(message),
            Self::TabLimit => formatter.write_str("The development tab limit is 32"),
            Self::HistoryLimit => {
                formatter.write_str("The development history limit is 128 entries")
            }
            Self::UnknownTab => formatter.write_str("Tab is no longer open"),
            Self::NoHistory => formatter.write_str("No page in that direction"),
            Self::CounterExhausted => formatter.write_str("Navigation identifier limit reached"),
        }
    }
}

impl std::error::Error for NavigationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    id: TabId,
    history: Vec<PageSource>,
    index: usize,
    generation: u64,
    status: LoadStatus,
}

impl Tab {
    pub fn id(&self) -> TabId {
        self.id
    }
    pub fn source(&self) -> &PageSource {
        &self.history[self.index]
    }
    pub fn status(&self) -> &LoadStatus {
        &self.status
    }
    pub fn request_key(&self) -> RequestKey {
        RequestKey {
            tab: self.id,
            generation: self.generation,
        }
    }
    pub fn can_back(&self) -> bool {
        self.index > 0
    }
    pub fn can_forward(&self) -> bool {
        self.index + 1 < self.history.len()
    }
    #[cfg(test)]
    pub fn history_len(&self) -> usize {
        self.history.len()
    }
    #[cfg(test)]
    pub fn history_index(&self) -> usize {
        self.index
    }

    fn initial(id: TabId, source: PageSource) -> Self {
        Self {
            id,
            history: vec![source],
            index: 0,
            generation: 1,
            status: LoadStatus::Loading,
        }
    }

    fn request(&self, reason: LoadReason) -> LoadRequest {
        LoadRequest {
            key: self.request_key(),
            source: self.source().clone(),
            reason,
        }
    }

    fn next_generation(&self) -> Result<u64, NavigationError> {
        self.generation
            .checked_add(1)
            .ok_or(NavigationError::CounterExhausted)
    }

    fn begin(&mut self, generation: u64, reason: LoadReason) -> LoadRequest {
        self.generation = generation;
        self.status = LoadStatus::Loading;
        self.request(reason)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Browser {
    tabs: Vec<Tab>,
    active: usize,
    next_tab_id: u64,
}

impl Browser {
    pub fn new(initial: PageSource) -> Result<(Self, LoadRequest), NavigationError> {
        initial.validate()?;
        let tab = Tab::initial(TabId(1), initial);
        let request = tab.request(LoadReason::Navigation);
        Ok((
            Self {
                tabs: vec![tab],
                active: 0,
                next_tab_id: 2,
            },
            request,
        ))
    }

    // Constructor and every close keep at least one nonempty history tab. Only
    // this module can mutate index/history, so active/source indexing is internal.
    pub fn active(&self) -> &Tab {
        &self.tabs[self.active]
    }
    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn new_tab(&mut self) -> Result<LoadRequest, NavigationError> {
        if self.tabs.len() >= MAX_TABS {
            return Err(NavigationError::TabLimit);
        }
        let next = self
            .next_tab_id
            .checked_add(1)
            .ok_or(NavigationError::CounterExhausted)?;
        let tab = Tab::initial(TabId(self.next_tab_id), PageSource::Welcome);
        let request = tab.request(LoadReason::Navigation);
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
        self.next_tab_id = next;
        Ok(request)
    }

    pub fn select(&mut self, id: TabId) -> Result<bool, NavigationError> {
        let index = self
            .tabs
            .iter()
            .position(|tab| tab.id == id)
            .ok_or(NavigationError::UnknownTab)?;
        let changed = index != self.active;
        self.active = index;
        Ok(changed)
    }

    pub fn select_relative(&mut self, direction: i32) -> bool {
        let next =
            (self.active as i64 + i64::from(direction)).rem_euclid(self.tabs.len() as i64) as usize;
        let changed = next != self.active;
        self.active = next;
        changed
    }

    /// Closing the final development tab resets to Welcome with a new identity.
    /// Other closes preserve neighboring tab state and return no new request.
    pub fn close(&mut self, id: TabId) -> Result<Option<LoadRequest>, NavigationError> {
        let index = self
            .tabs
            .iter()
            .position(|tab| tab.id == id)
            .ok_or(NavigationError::UnknownTab)?;
        if self.tabs.len() == 1 {
            let next = self
                .next_tab_id
                .checked_add(1)
                .ok_or(NavigationError::CounterExhausted)?;
            let tab = Tab::initial(TabId(self.next_tab_id), PageSource::Welcome);
            let request = tab.request(LoadReason::Navigation);
            self.tabs[0] = tab;
            self.active = 0;
            self.next_tab_id = next;
            return Ok(Some(request));
        }
        self.tabs.remove(index);
        if index < self.active {
            self.active -= 1;
        } else if index == self.active {
            self.active = index.min(self.tabs.len() - 1);
        }
        Ok(None)
    }

    pub fn navigate(&mut self, source: PageSource) -> Result<LoadRequest, NavigationError> {
        source.validate()?;
        let tab = &mut self.tabs[self.active];
        if tab.source() == &source {
            let generation = tab.next_generation()?;
            return Ok(tab.begin(generation, LoadReason::Reload));
        }
        if tab.index + 2 > MAX_HISTORY {
            return Err(NavigationError::HistoryLimit);
        }
        // Validate all fallible preconditions before truncating the forward branch.
        let generation = tab.next_generation()?;
        tab.history.truncate(tab.index + 1);
        tab.history.push(source);
        tab.index += 1;
        Ok(tab.begin(generation, LoadReason::Navigation))
    }

    pub fn back(&mut self) -> Result<LoadRequest, NavigationError> {
        let tab = &mut self.tabs[self.active];
        if !tab.can_back() {
            return Err(NavigationError::NoHistory);
        }
        let generation = tab.next_generation()?;
        tab.index -= 1;
        Ok(tab.begin(generation, LoadReason::Navigation))
    }

    pub fn forward(&mut self) -> Result<LoadRequest, NavigationError> {
        let tab = &mut self.tabs[self.active];
        if !tab.can_forward() {
            return Err(NavigationError::NoHistory);
        }
        let generation = tab.next_generation()?;
        tab.index += 1;
        Ok(tab.begin(generation, LoadReason::Navigation))
    }

    pub fn reload(&mut self) -> Result<LoadRequest, NavigationError> {
        let tab = &mut self.tabs[self.active];
        let generation = tab.next_generation()?;
        Ok(tab.begin(generation, LoadReason::Reload))
    }

    pub fn rerender(&mut self) -> Result<LoadRequest, NavigationError> {
        let tab = &mut self.tabs[self.active];
        let generation = tab.next_generation()?;
        Ok(tab.begin(generation, LoadReason::Reflow))
    }

    /// Only the current pending generation can finish, including for inactive tabs.
    /// The caller must apply the corresponding display under this same token.
    pub fn complete(&mut self, key: RequestKey, result: Result<(), String>) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == key.tab) else {
            return false;
        };
        if tab.generation != key.generation || tab.status != LoadStatus::Loading {
            return false;
        }
        tab.status = match result {
            Ok(()) => LoadStatus::Ready,
            Err(mut message) => {
                let mut end = message.len().min(MAX_FAILURE_BYTES);
                while !message.is_char_boundary(end) {
                    end -= 1;
                }
                message.truncate(end);
                if message.is_empty() {
                    message.push_str("Unable to load page.");
                }
                LoadStatus::Failed(message)
            }
        };
        true
    }

    pub fn is_current(&self, key: RequestKey) -> bool {
        self.tabs.iter().any(|tab| tab.request_key() == key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn browser() -> (Browser, LoadRequest) {
        Browser::new(PageSource::Welcome).unwrap()
    }

    #[test]
    fn navigation_and_history_keep_loading_identity_and_discard_forward_branch() {
        let (mut model, initial) = browser();
        assert_eq!(model.active().source(), &PageSource::Welcome);
        assert_eq!(model.active().status(), &LoadStatus::Loading);
        assert!(model.complete(initial.key, Ok(())));
        let guide = model.navigate(PageSource::Guide).unwrap();
        assert_eq!(guide.reason, LoadReason::Navigation);
        assert_eq!(model.active().source(), &PageSource::Guide);
        assert_eq!(model.active().status(), &LoadStatus::Loading);
        assert!(model.active().can_back());
        assert_eq!(model.active().history_index(), 1);
        let back = model.back().unwrap();
        assert_eq!(back.source, PageSource::Welcome);
        assert_eq!(model.active().history_index(), 0);
        assert!(model.active().can_forward());
        let replacement = PageSource::Local(PathBuf::from("replacement.html"));
        model.navigate(replacement.clone()).unwrap();
        assert_eq!(model.active().source(), &replacement);
        assert_eq!(model.active().history_len(), 2);
        assert!(!model.active().can_forward());
        assert_eq!(model.forward(), Err(NavigationError::NoHistory));
    }

    #[test]
    fn reload_same_address_and_reflow_do_not_duplicate_history() {
        let (mut model, _) = browser();
        let reload = model.navigate(PageSource::Welcome).unwrap();
        assert_eq!(reload.reason, LoadReason::Reload);
        let explicit_reload = model.reload().unwrap();
        assert_eq!(explicit_reload.reason, LoadReason::Reload);
        let reflow = model.rerender().unwrap();
        assert_eq!(reflow.reason, LoadReason::Reflow);
        assert_eq!(model.active().history_len(), 1);
        assert!(reload.key.generation < explicit_reload.key.generation);
        assert!(explicit_reload.key.generation < reflow.key.generation);
        assert!(!model.is_current(reload.key));
        assert!(model.is_current(reflow.key));
    }

    #[test]
    fn stale_duplicate_and_closed_tab_completions_cannot_replace_current_state() {
        let (mut model, old) = browser();
        let current = model.navigate(PageSource::Guide).unwrap();
        assert!(!model.complete(old.key, Ok(())));
        assert!(model.complete(current.key, Ok(())));
        assert!(!model.complete(current.key, Err("late duplicate".to_owned())));
        assert_eq!(model.active().status(), &LoadStatus::Ready);
        let other = model.new_tab().unwrap();
        model.close(current.key.tab).unwrap();
        assert!(!model.complete(current.key, Ok(())));
        assert!(model.complete(other.key, Ok(())));
    }

    #[test]
    fn closing_last_tab_creates_new_welcome_identity() {
        let (mut model, original) = browser();
        let reset = model.close(original.key.tab).unwrap().unwrap();
        assert_eq!(model.tabs().len(), 1);
        assert_eq!(reset.source, PageSource::Welcome);
        assert!(reset.key.tab > original.key.tab);
        assert!(!model.complete(original.key, Ok(())));
        assert!(model.complete(reset.key, Ok(())));
    }

    #[test]
    fn tab_selection_wraps_and_closure_preserves_neighbor_and_pending_results() {
        let (mut model, first) = browser();
        let second = model.new_tab().unwrap();
        let third = model.new_tab().unwrap();
        model.select(second.key.tab).unwrap();
        model.close(second.key.tab).unwrap();
        assert_eq!(model.active().id(), third.key.tab);
        model.select_relative(1);
        assert_eq!(model.active().id(), first.key.tab);
        model.select_relative(-1);
        assert_eq!(model.active().id(), third.key.tab);
        let active = model.active().id();
        assert!(model.complete(first.key, Ok(())));
        assert_eq!(model.active().id(), active);
        assert_eq!(
            model.select(TabId(u64::MAX)),
            Err(NavigationError::UnknownTab)
        );
    }

    #[test]
    fn tab_and_history_limits_fail_without_mutating_state() {
        let (mut model, _) = browser();
        for _ in 1..MAX_TABS {
            model.new_tab().unwrap();
        }
        let full = model.clone();
        assert_eq!(model.new_tab(), Err(NavigationError::TabLimit));
        assert_eq!(model, full);
        let (mut model, _) = browser();
        for number in 1..MAX_HISTORY {
            model
                .navigate(PageSource::Local(PathBuf::from(format!(
                    "page-{number}.html"
                ))))
                .unwrap();
        }
        let full = model.clone();
        assert_eq!(
            model.navigate(PageSource::Guide),
            Err(NavigationError::HistoryLimit)
        );
        assert_eq!(model, full);
        model.back().unwrap();
        model.navigate(PageSource::Guide).unwrap();
        assert_eq!(model.active().history_len(), MAX_HISTORY);
        assert!(!model.active().can_forward());
    }

    #[test]
    fn request_and_tab_counter_exhaustion_are_atomic_failures() {
        let (mut model, _) = browser();
        model.tabs[0].generation = u64::MAX;
        let snapshot = model.clone();
        assert_eq!(
            model.navigate(PageSource::Guide),
            Err(NavigationError::CounterExhausted)
        );
        assert_eq!(model, snapshot);
        model.next_tab_id = u64::MAX;
        let snapshot = model.clone();
        assert_eq!(model.new_tab(), Err(NavigationError::CounterExhausted));
        assert_eq!(
            model.close(model.active().id()),
            Err(NavigationError::CounterExhausted)
        );
        assert_eq!(model, snapshot);
    }

    #[test]
    fn source_validation_rejects_active_schemes_devices_controls_and_invalid_direct_sources() {
        assert_eq!(
            PageSource::from_address(" about:welcome ").unwrap(),
            PageSource::Welcome
        );
        assert_eq!(
            PageSource::from_address("about:guide").unwrap(),
            PageSource::Guide
        );
        assert_eq!(PageSource::Welcome.title(), "New tab");
        assert_eq!(PageSource::Guide.title(), "Guide");
        assert_eq!(
            PageSource::Local(PathBuf::from("document.html")).title(),
            "document.html"
        );
        assert_eq!(
            PageSource::from_address("document.html").unwrap().address(),
            "document.html"
        );
        for address in [
            "https://example.invalid",
            "file:///page.html",
            "about:unknown",
            "NUL",
            "C:relative.html",
            "//share/page",
            "page\0.html",
            "page\nname.html",
            "page\u{202e}.html",
            "\tpage.html",
            "page.html\r",
            "\nabout:welcome",
            "",
        ] {
            assert!(
                PageSource::from_address(address).is_err(),
                "unexpected accepted source"
            );
        }
        assert!(PageSource::from_address(&"x".repeat(MAX_ADDRESS_BYTES + 1)).is_err());
        assert!(PageSource::from_address(&"x".repeat(MAX_ADDRESS_BYTES)).is_ok());
        let (mut model, _) = browser();
        let snapshot = model.clone();
        assert!(
            model
                .navigate(PageSource::Local(PathBuf::from("https://example.invalid")))
                .is_err()
        );
        assert_eq!(model, snapshot);
    }

    #[test]
    fn failure_messages_remain_bounded_utf8_and_do_not_revert_source() {
        let (mut model, _) = browser();
        let request = model.navigate(PageSource::Guide).unwrap();
        assert!(model.complete(request.key, Err(format!("abc{}", "🌲".repeat(1024)))));
        let LoadStatus::Failed(message) = model.active().status() else {
            panic!("missing failure state");
        };
        assert_eq!(message.len(), MAX_FAILURE_BYTES - 1);
        assert!(message.ends_with('🌲'));
        assert_eq!(model.active().source(), &PageSource::Guide);
        assert!(!model.complete(request.key, Ok(())));
    }

    #[test]
    fn no_history_unknown_tab_and_invalid_initial_source_are_explicit_failures() {
        let (mut model, _) = browser();
        let snapshot = model.clone();
        assert_eq!(model.back(), Err(NavigationError::NoHistory));
        assert_eq!(model.forward(), Err(NavigationError::NoHistory));
        assert_eq!(model.close(TabId(0)), Err(NavigationError::UnknownTab));
        assert_eq!(model.select(TabId(0)), Err(NavigationError::UnknownTab));
        assert_eq!(model, snapshot);
        assert!(Browser::new(PageSource::Local(PathBuf::from("NUL"))).is_err());
        assert!(model.complete(model.active().request_key(), Err(String::new())));
        assert_eq!(
            model.active().status(),
            &LoadStatus::Failed("Unable to load page.".to_owned())
        );
    }

    #[test]
    fn local_filename_spelling_round_trips_without_silent_space_normalization() {
        for name in [
            " leading.html",
            "document with spaces.html",
            "trailing.html ",
        ] {
            let source = PageSource::Local(PathBuf::from(name));
            let (model, _) = Browser::new(source.clone()).unwrap();
            assert_eq!(
                PageSource::from_address(&model.active().source().address()).unwrap(),
                source
            );
        }
        assert!(PageSource::from_address("   ").is_err());
        assert!(
            PageSource::from_address(&format!("{}about:welcome", " ".repeat(MAX_ADDRESS_BYTES)))
                .is_err()
        );
    }

    #[test]
    fn generated_transition_sequences_preserve_identity_history_and_completion_invariants() {
        let (mut model, initial) = browser();
        let mut requests = vec![initial.key];
        let mut sequence = 0x519c_8d17_u64;
        for step in 0..1024 {
            sequence = sequence
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let request = match (sequence >> 32) % 10 {
                0 => model.new_tab().ok(),
                1 => model.back().ok(),
                2 => model.forward().ok(),
                3 => model
                    .navigate(PageSource::Local(PathBuf::from(format!(
                        "page-{}.html",
                        step % 37
                    ))))
                    .ok(),
                4 => model.reload().ok(),
                5 => model.rerender().ok(),
                6 => {
                    model.select_relative(-1);
                    None
                }
                7 => {
                    model.select_relative(1);
                    None
                }
                8 => model.close(model.active().id()).unwrap(),
                _ => {
                    let key = requests[(sequence as usize) % requests.len()];
                    let eligible = model.tabs().iter().any(|tab| {
                        tab.request_key() == key && tab.status() == &LoadStatus::Loading
                    });
                    assert_eq!(model.complete(key, Ok(())), eligible);
                    None
                }
            };
            if let Some(request) = request {
                assert!(model.is_current(request.key));
                assert_eq!(model.active().source(), &request.source);
                assert_eq!(model.active().status(), &LoadStatus::Loading);
                requests.push(request.key);
            }
            assert!((1..=MAX_TABS).contains(&model.tabs().len()));
            assert!(
                model
                    .tabs()
                    .iter()
                    .any(|tab| tab.id() == model.active().id())
            );
            let identities: std::collections::HashSet<_> =
                model.tabs().iter().map(Tab::id).collect();
            assert_eq!(identities.len(), model.tabs().len());
            for tab in model.tabs() {
                assert!((1..=MAX_HISTORY).contains(&tab.history_len()));
                assert!(tab.history_index() < tab.history_len());
                assert!(tab.id().0 < model.next_tab_id);
                assert_eq!(
                    PageSource::from_address(&tab.source().address()).unwrap(),
                    *tab.source()
                );
            }
        }
    }
}

#![forbid(unsafe_code)]

//! Bounded background loading for the local application. The UI owns navigation
//! authority; this scheduler coalesces reflows and never interprets HTML itself.

use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
};

use opensequoia_engine::DisplayList;

use crate::browser::{LoadReason, LoadRequest, MAX_TABS, PageSource, RequestKey, TabId};

pub struct RenderResult {
    pub key: RequestKey,
    pub result: Result<DisplayList, String>,
}

struct Job {
    request: LoadRequest,
    width: u32,
}

struct CachedHtml {
    source: PageSource,
    html: Arc<str>,
}

#[derive(Default)]
struct State {
    pending: BTreeMap<TabId, Job>,
    order: VecDeque<TabId>,
    latest: BTreeMap<TabId, RequestKey>,
    results: BTreeMap<TabId, RenderResult>,
    html: BTreeMap<TabId, CachedHtml>,
    stopping: bool,
}

struct Shared {
    state: Mutex<State>,
    wake: Condvar,
}

pub struct RenderController {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl RenderController {
    pub fn new() -> Result<Self, String> {
        Self::start(crate::worker::render)
    }

    fn start(
        render: impl Fn(&str, u32) -> Result<DisplayList, String> + Send + 'static,
    ) -> Result<Self, String> {
        let shared = Arc::new(Shared {
            state: Mutex::new(State::default()),
            wake: Condvar::new(),
        });
        let background = Arc::clone(&shared);
        let thread = thread::Builder::new()
            .name("page-render-controller".to_owned())
            .spawn(move || run(&background, render))
            .map_err(|_| "Could not start page loading".to_owned())?;
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }

    /// Only model-generated live-tab requests should be submitted. Replacing a
    /// pending request for the same tab keeps its queue position and memory bound.
    pub fn submit(&self, request: LoadRequest, width: u32) -> Result<(), String> {
        if !(64..=8192).contains(&width) || request.key.generation == 0 || request.key.tab.0 == 0 {
            return Err("Invalid page rendering request".to_owned());
        }
        if self.thread.as_ref().is_none_or(JoinHandle::is_finished) {
            return Err("Page loading stopped".to_owned());
        }
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| "Page loading stopped".to_owned())?;
        if state.stopping {
            return Err("Page loading stopped".to_owned());
        }
        let tab = request.key.tab;
        if !state.latest.contains_key(&tab) && state.latest.len() >= MAX_TABS {
            return Err("Too many pages are loading".to_owned());
        }
        if state
            .latest
            .get(&tab)
            .is_some_and(|key| request.key.generation < key.generation)
        {
            return Err("Page request was replaced".to_owned());
        }
        state.latest.insert(tab, request.key);
        state.results.remove(&tab);
        if request.reason != LoadReason::Reflow {
            state.html.remove(&tab);
        }
        if !state.pending.contains_key(&tab) {
            state.order.push_back(tab);
        }
        state.pending.insert(tab, Job { request, width });
        drop(state);
        self.shared.wake.notify_one();
        Ok(())
    }

    /// Drain is nonblocking with respect to file access or renderer execution.
    /// The caller must also use Browser::complete to validate the live generation.
    pub fn drain(&self) -> Vec<RenderResult> {
        let (mut state, poisoned) = match self.shared.state.lock() {
            Ok(state) => (state, false),
            Err(error) => (error.into_inner(), true),
        };
        let stopped = self.thread.as_ref().is_none_or(JoinHandle::is_finished);
        if !state.stopping && (poisoned || stopped) {
            state.stopping = true;
            state.pending.clear();
            state.order.clear();
            state.html.clear();
            let keys: Vec<_> = state.latest.values().copied().collect();
            for key in keys {
                state
                    .results
                    .entry(key.tab)
                    .or_insert_with(|| RenderResult {
                        key,
                        result: Err("Page loading stopped".to_owned()),
                    });
            }
        }
        std::mem::take(&mut state.results).into_values().collect()
    }

    /// Remove all pending/cache/result authority for a closed tab. A running
    /// bounded renderer may finish, but its reply cannot be published afterward.
    pub fn cancel(&self, tab: TabId) {
        if let Ok(mut state) = self.shared.state.lock() {
            state.latest.remove(&tab);
            state.pending.remove(&tab);
            state.order.retain(|queued| *queued != tab);
            state.results.remove(&tab);
            state.html.remove(&tab);
        }
    }
}

impl Drop for RenderController {
    fn drop(&mut self) {
        if let Ok(mut state) = self.shared.state.lock() {
            state.stopping = true;
            state.pending.clear();
            state.order.clear();
            state.latest.clear();
            state.results.clear();
            state.html.clear();
        }
        self.shared.wake.notify_one();
        if let Some(thread) = self.thread.take()
            && thread.is_finished()
        {
            let _ = thread.join();
        }
        // An active bounded child call is not joined on the UI thread. It observes
        // stopping after returning. Parent-death/process-tree cleanup is an OS Job
        // responsibility, not a guarantee supplied by this scheduling primitive.
    }
}

fn read_source(source: &PageSource) -> Result<Arc<str>, String> {
    let html = match source {
        PageSource::Welcome => crate::input::load(None)?,
        PageSource::Guide => include_str!("../../../fixtures/guide.html").to_owned(),
        PageSource::Local(path) => crate::input::load(Some(path))?,
    };
    Ok(Arc::from(html))
}

fn run(shared: &Shared, render: impl Fn(&str, u32) -> Result<DisplayList, String>) {
    loop {
        let (job, cached) = {
            let Ok(mut state) = shared.state.lock() else {
                return;
            };
            while state.order.is_empty() && !state.stopping {
                let Ok(awakened) = shared.wake.wait(state) else {
                    return;
                };
                state = awakened;
            }
            if state.stopping {
                return;
            }
            let Some(tab) = state.order.pop_front() else {
                continue;
            };
            let Some(job) = state.pending.remove(&tab) else {
                continue;
            };
            if state.latest.get(&tab) != Some(&job.request.key) {
                continue;
            }
            let cached = if job.request.reason == LoadReason::Reflow {
                state
                    .html
                    .get(&tab)
                    .filter(|cache| cache.source == job.request.source)
                    .map(|cache| Arc::clone(&cache.html))
            } else {
                None
            };
            (job, cached)
        };
        let html = cached.map_or_else(|| read_source(&job.request.source), Ok);
        // All file reads, process creation, parsing, layout and waits happen here.
        let result = html
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|html| render(html, job.width));
        let Ok(mut state) = shared.state.lock() else {
            return;
        };
        if state.stopping {
            return;
        }
        if state.latest.get(&job.request.key.tab) != Some(&job.request.key) {
            continue;
        }
        if result.is_ok()
            && let Ok(html) = html
        {
            state.html.insert(
                job.request.key.tab,
                CachedHtml {
                    source: job.request.source,
                    html,
                },
            );
        }
        state.results.insert(
            job.request.key.tab,
            RenderResult {
                key: job.request.key,
                result,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::Browser;
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };

    fn empty_display(width: u32) -> DisplayList {
        DisplayList {
            width,
            height: 32,
            items: Vec::new(),
        }
    }

    fn await_result(controller: &RenderController) -> RenderResult {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(result) = controller.drain().pop() {
                return result;
            }
            assert!(
                Instant::now() < deadline,
                "controller did not produce a bounded reply"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn pending_reflows_coalesce_and_running_stale_reply_is_not_published() {
        let (begun, start) = mpsc::channel();
        let (release, gate) = mpsc::channel();
        let controller = RenderController::start(move |_, width| {
            begun.send(width).unwrap();
            gate.recv_timeout(Duration::from_secs(3)).unwrap();
            Ok(empty_display(width))
        })
        .unwrap();
        let (mut browser, first) = Browser::new(PageSource::Welcome).unwrap();
        controller.submit(first, 640).unwrap();
        assert_eq!(start.recv_timeout(Duration::from_secs(3)).unwrap(), 640);
        let discarded = browser.rerender().unwrap();
        controller.submit(discarded, 700).unwrap();
        let last = browser.rerender().unwrap();
        controller.submit(last.clone(), 800).unwrap();
        release.send(()).unwrap();
        assert_eq!(start.recv_timeout(Duration::from_secs(3)).unwrap(), 800);
        // The next render has begun but cannot finish yet: the first producer's
        // stale publication would now be observable instead of being overwritten.
        let premature = controller.drain();
        release.send(()).unwrap();
        let result = await_result(&controller);
        assert!(premature.is_empty(), "stale result became observable");
        assert_eq!(result.key, last.key);
        assert_eq!(result.result.unwrap().width, 800);
        assert!(controller.drain().is_empty());
    }

    #[test]
    fn closing_a_tab_discards_its_late_reply_and_frees_capacity() {
        let (begun, start) = mpsc::channel();
        let (release, gate) = mpsc::channel();
        let controller = RenderController::start(move |_, width| {
            begun.send(()).unwrap();
            gate.recv_timeout(Duration::from_secs(3)).unwrap();
            Ok(empty_display(width))
        })
        .unwrap();
        let (mut browser, first) = Browser::new(PageSource::Welcome).unwrap();
        controller.submit(first.clone(), 640).unwrap();
        start.recv_timeout(Duration::from_secs(3)).unwrap();
        controller.cancel(first.key.tab);
        let next = browser.close(first.key.tab).unwrap().unwrap();
        controller.submit(next.clone(), 800).unwrap();
        release.send(()).unwrap();
        // A sequential second producer proves the first result publication path
        // has completed. Keep it blocked while checking the observable queue.
        start.recv_timeout(Duration::from_secs(3)).unwrap();
        let late = controller.drain();
        release.send(()).unwrap();
        let result = await_result(&controller);
        assert!(late.is_empty(), "closed tab published a late result");
        assert_eq!(result.key, next.key);
        assert_eq!(result.result.unwrap().width, 800);
    }

    #[test]
    fn renderer_failure_is_reported_for_the_current_request() {
        let controller = RenderController::start(|_, _| Err("fixture failure".to_owned())).unwrap();
        let (_, first) = Browser::new(PageSource::Guide).unwrap();
        controller.submit(first.clone(), 960).unwrap();
        let result = await_result(&controller);
        assert_eq!(result.key, first.key);
        assert_eq!(result.result.unwrap_err(), "fixture failure");
    }

    #[test]
    fn scheduler_capacity_and_viewport_checks_do_not_mutate_state() {
        let controller = RenderController::start(|_, width| Ok(empty_display(width))).unwrap();
        for id in 1..=MAX_TABS as u64 {
            controller
                .submit(
                    LoadRequest {
                        key: RequestKey {
                            tab: TabId(id),
                            generation: 1,
                        },
                        source: PageSource::Welcome,
                        reason: LoadReason::Navigation,
                    },
                    960,
                )
                .unwrap();
        }
        let overflow = LoadRequest {
            key: RequestKey {
                tab: TabId(MAX_TABS as u64 + 1),
                generation: 1,
            },
            source: PageSource::Welcome,
            reason: LoadReason::Navigation,
        };
        assert!(controller.submit(overflow.clone(), 960).is_err());
        assert!(controller.submit(overflow, 0).is_err());
        let state = controller.shared.state.lock().unwrap();
        assert_eq!(state.latest.len(), MAX_TABS);
        assert!(state.pending.len() <= MAX_TABS && state.results.len() <= MAX_TABS);
    }
}

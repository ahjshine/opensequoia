//! Native browser coordinator. DOM/file/process work runs in the controller.
//! HWND data never holds Rust pointers; borrows end before reentrant OS calls.

use crate::{
    browser::{Browser, LoadReason, LoadRequest, LoadStatus, PageSource, TabId},
    chrome::{self, Action, Chrome, Snapshot},
    controller::{RenderController, RenderResult},
};
use opensequoia_engine::DisplayList;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    panic::AssertUnwindSafe,
    path::{Component, Path, Prefix},
    ptr,
    rc::Rc,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::*,
    Storage::FileSystem::GetDriveTypeW,
    System::LibraryLoader::GetModuleHandleW,
    UI::{HiDpi::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

const POLL_TIMER: usize = 1;
const REFLOW_TIMER: usize = 2;
const CSS_DPI: u32 = 96;
struct State {
    browser: Browser,
    controller: RenderController,
    initial: Option<LoadRequest>,
    chrome: Option<Rc<Chrome>>,
    frames: BTreeMap<TabId, Rc<DisplayList>>,
    scrolls: BTreeMap<TabId, i32>,
    width: u32,
    height: u32,
    dpi: u32,
    reflow_pending: bool,
    notice: Option<String>,
    failure: Option<String>,
    smoke: Smoke,
    started: Instant,
    paints: u32,
    total_paints: u32,
    text_runs: u32,
}
#[derive(Clone, Copy)]
enum Smoke {
    None,
    Paint,
    App { stage: u8, first: TabId },
}
thread_local! {static STATE:RefCell<Option<State>>=const {RefCell::new(None)};}
fn state<T>(f: impl FnOnce(&mut State) -> T) -> Option<T> {
    STATE.with(|cell| cell.try_borrow_mut().ok()?.as_mut().map(f))
}
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
fn os_error(action: &str) -> String {
    format!("{action}: {}", std::io::Error::last_os_error())
}

pub fn run(source: PageSource, smoke: bool) -> Result<(), String> {
    run_mode(source, smoke.then_some(false))
}
pub fn app_smoke() -> Result<(), String> {
    run_mode(PageSource::Welcome, Some(true))
}
fn run_mode(source: PageSource, smoke: Option<bool>) -> Result<(), String> {
    let (browser, initial) = Browser::new(source).map_err(|e| e.to_string())?;
    let first = browser.active().id();
    let controller = RenderController::new()?;
    STATE.with(|slot| {
        *slot.borrow_mut() = Some(State {
            browser,
            controller,
            initial: Some(initial),
            chrome: None,
            frames: BTreeMap::new(),
            scrolls: BTreeMap::new(),
            width: 960,
            height: 640,
            dpi: CSS_DPI,
            reflow_pending: false,
            notice: None,
            failure: None,
            started: Instant::now(),
            paints: 0,
            total_paints: 0,
            text_runs: 0,
            smoke: match smoke {
                None => Smoke::None,
                Some(false) => Smoke::Paint,
                Some(true) => Smoke::App { stage: 0, first },
            },
        })
    });
    let result = native_loop();
    let final_state = STATE
        .with(|slot| slot.borrow_mut().take())
        .ok_or("native state was lost")?;
    result?;
    if let Some(error) = final_state.failure {
        return Err(error);
    }
    match final_state.smoke {
        Smoke::None => {}
        Smoke::Paint => {
            if final_state.paints == 0 || final_state.text_runs == 0 {
                return Err("no successful child-rendered native paint".to_owned());
            }
            println!(
                "WINDOW_SMOKE_OK paints={} drawn_text_runs={} child_rendered=1",
                final_state.paints, final_state.text_runs
            );
        }
        Smoke::App { stage, .. } => {
            if stage != 10 {
                return Err(format!("application smoke did not finish (stage {stage})"));
            }
            println!(
                "APP_SMOKE_OK navigation=1 back=1 forward=1 tabs=1 close_reset=1 native_controls=1 child_rendered=1 paints={}",
                final_state.total_paints
            );
        }
    }
    Ok(())
}

fn native_loop() -> Result<(), String> {
    let class_name = wide("OpenSequoia.BrowserWindow");
    let title = wide("OpenSequoia");
    // SAFETY: Initialized Win32 values, static callback and terminated buffers
    // stay live through teardown. No Rust pointer is placed in window data.
    unsafe {
        let awareness = DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2;
        if SetProcessDpiAwarenessContext(awareness) == 0
            && AreDpiAwarenessContextsEqual(GetThreadDpiAwarenessContext(), awareness) == 0
        {
            return Err(os_error("per-monitor DPI awareness failed"));
        }
        let module = GetModuleHandleW(ptr::null());
        if module.is_null() {
            return Err(os_error("module handle failed"));
        }
        let cursor = LoadCursorW(ptr::null_mut(), IDC_ARROW);
        if cursor.is_null() {
            return Err(os_error("cursor failed"));
        }
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: module,
            hCursor: cursor,
            lpszClassName: class_name.as_ptr(),
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            return Err(os_error("window class registration failed"));
        }
        let hwnd = CreateWindowExW(
            WS_EX_CONTROLPARENT,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1100,
            780,
            ptr::null_mut(),
            ptr::null_mut(),
            module,
            ptr::null(),
        );
        if hwnd.is_null() {
            let error = os_error("native window creation failed");
            UnregisterClassW(class_name.as_ptr(), module);
            return Err(error);
        }
        let mut accelerator = ptr::null_mut();
        let result = (|| {
            let dpi = GetDpiForWindow(hwnd);
            if dpi == 0 {
                return Err(os_error("window DPI failed"));
            }
            let controls = Rc::new(Chrome::create(hwnd, dpi)?);
            state(|s| s.chrome = Some(controls)).ok_or("native state unavailable")?;
            accelerator = chrome::accelerators()?;
            resize(hwnd)?;
            let initial = state(|s| s.initial.take())
                .flatten()
                .ok_or("initial navigation unavailable")?;
            queue(hwnd, initial)?;
            if SetTimer(hwnd, POLL_TIMER, 30, None) == 0 {
                return Err(os_error("render polling timer failed"));
            }
            ShowWindow(hwnd, SW_SHOW);
            if UpdateWindow(hwnd) == 0 {
                return Err(os_error("initial window paint failed"));
            }
            let mut message = MSG::default();
            loop {
                let status = GetMessageW(&mut message, ptr::null_mut(), 0, 0);
                if status < 0 {
                    return Err(os_error("native message retrieval failed"));
                }
                if status == 0 {
                    return if message.wParam == 0 {
                        Ok(())
                    } else {
                        Err("native loop failed".to_owned())
                    };
                }
                if message.message == WM_KEYDOWN {
                    if message.wParam == usize::from(VK_ESCAPE) {
                        close_window(hwnd)?;
                        continue;
                    }
                    if message.wParam == usize::from(VK_RETURN)
                        && let Some(controls) = state(|s| s.chrome.clone()).flatten()
                    {
                        if controls.editor_focused() {
                            act(hwnd, Action::Go)?;
                            continue;
                        }
                        if controls.click_focused() {
                            continue;
                        }
                    }
                }
                if TranslateAcceleratorW(hwnd, accelerator, &message) != 0 {
                    continue;
                }
                if IsDialogMessageW(hwnd, &message) != 0 {
                    continue;
                }
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        })();
        // Parent destruction precedes dropping controls/fonts. No Rust borrow
        // is held across teardown, which can synchronously call wnd_proc.
        let destroyed = IsWindow(hwnd) == 0 || DestroyWindow(hwnd) != 0;
        let accel_destroyed = accelerator.is_null() || DestroyAcceleratorTable(accelerator) != 0;
        let unregistered = UnregisterClassW(class_name.as_ptr(), module) != 0;
        result?;
        if !destroyed || !accel_destroyed || !unregistered {
            return Err("native resource teardown failed".to_owned());
        }
        Ok(())
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match std::panic::catch_unwind(AssertUnwindSafe(|| {
        on_message(hwnd, message, wparam, lparam)
    })) {
        Ok(result) => result,
        Err(_) => {
            state(|s| s.failure = Some("panic in native callback".to_owned()));
            // SAFETY: UI-thread quit signal with no pointer arguments.
            unsafe { PostQuitMessage(1) };
            0
        }
    }
}
fn fail(hwnd: HWND, error: String) {
    state(|s| {
        if s.failure.is_none() {
            s.failure = Some(error)
        }
    });
    if close_window(hwnd).is_err() {
        unsafe { PostQuitMessage(1) };
    }
}
fn close_window(hwnd: HWND) -> Result<(), String> {
    // SAFETY: Own window, numeric asynchronous message without pointer payload.
    if unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) } == 0 {
        Err(os_error("close post failed"))
    } else {
        Ok(())
    }
}

fn on_message(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_PAINT => {
            let snapshot = state(|s| {
                let id = s.browser.active().id();
                (
                    s.frames.get(&id).cloned(),
                    *s.scrolls.get(&id).unwrap_or(&0),
                    s.dpi,
                    status_text(s),
                )
            });
            if let Some((display, scroll, dpi, status)) = snapshot {
                match chrome::paint(hwnd, display.as_deref(), scroll, dpi, &status) {
                    Ok(count) => {
                        let close = state(|s| {
                            if count > 0 {
                                s.paints = s.paints.saturating_add(1);
                                s.total_paints = s.total_paints.saturating_add(1);
                                s.text_runs = s.text_runs.saturating_add(count);
                            }
                            matches!(s.smoke, Smoke::Paint) && count > 0
                        })
                        .unwrap_or(false);
                        if close && let Err(e) = close_window(hwnd) {
                            fail(hwnd, e);
                        }
                    }
                    Err(e) => fail(hwnd, e),
                }
                0
            } else {
                unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
            }
        }
        WM_SIZE | WM_DPICHANGED => {
            if let Err(e) = resize(hwnd) {
                fail(hwnd, e);
            }
            0
        }
        WM_TIMER if wparam == POLL_TIMER => {
            if let Err(e) = poll(hwnd).and_then(|()| smoke_tick(hwnd)) {
                fail(hwnd, e);
            }
            0
        }
        WM_TIMER if wparam == REFLOW_TIMER => {
            let pending = state(|s| {
                let p = s.reflow_pending;
                s.reflow_pending = false;
                p
            })
            .unwrap_or(false);
            // SAFETY: Own timer ID; no pointer/borrow survives the OS call.
            unsafe { KillTimer(hwnd, REFLOW_TIMER) };
            if pending && let Err(e) = rerender(hwnd) {
                fail(hwnd, e);
            }
            0
        }
        WM_COMMAND => {
            let id = (wparam & 0xffff) as u16;
            let notification = ((wparam >> 16) & 0xffff) as u16;
            let action = if lparam == 0 && notification == 1 {
                Chrome::accelerator(id)
            } else {
                state(|s| {
                    s.chrome
                        .as_ref()
                        .and_then(|c| c.command(id, notification, lparam as HWND))
                })
                .flatten()
            };
            if let Some(action) = action
                && let Err(e) = act(hwnd, action)
            {
                fail(hwnd, e);
            }
            0
        }
        WM_LBUTTONDOWN => {
            let y = ((lparam as usize >> 16) & 0xffff) as u16 as i16 as i32;
            let top = state(|s| chrome::scaled(chrome::TOP, s.dpi).unwrap_or(i32::MAX))
                .unwrap_or(i32::MAX);
            // SAFETY: Set focus only to own window, no parameter pointer is read.
            if y >= top {
                unsafe { SetFocus(hwnd) };
            }
            0
        }
        WM_MOUSEWHEEL => {
            let editing = state(|s| s.chrome.clone())
                .flatten()
                .is_some_and(|c| c.editor_focused());
            if !editing {
                let delta = ((wparam >> 16) & 0xffff) as u16 as i16 as i32;
                scroll_by(hwnd, -(delta / 120) * 48);
            }
            0
        }
        WM_KEYDOWN => {
            let amount = match wparam as u16 {
                VK_DOWN => Some(48),
                VK_UP => Some(-48),
                VK_HOME => Some(i32::MIN),
                VK_END => Some(i32::MAX),
                VK_NEXT => state(|s| i32::try_from(s.height).unwrap_or(i32::MAX)),
                VK_PRIOR => state(|s| -i32::try_from(s.height).unwrap_or(i32::MAX)),
                _ => None,
            };
            if let Some(amount) = amount {
                scroll_by(hwnd, amount);
            }
            0
        }
        WM_CLOSE => {
            // SAFETY: Destroy our UI-thread HWND; no borrowed Rust state is live.
            unsafe {
                if DestroyWindow(hwnd) == 0 {
                    state(|s| s.failure = Some(os_error("window destruction failed")));
                    PostQuitMessage(1);
                }
            }
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn status_text(s: &State) -> String {
    if let Some(notice) = &s.notice {
        return notice.clone();
    }
    match s.browser.active().status() {
        LoadStatus::Loading => "Loading local page…".to_owned(),
        LoadStatus::Ready => "Local HTML • Ready".to_owned(),
        LoadStatus::Failed(e) => format!(
            "Page could not load: {}",
            e.chars().take(160).collect::<String>()
        ),
    }
}
fn refresh(hwnd: HWND, address: bool) -> Result<(), String> {
    let snapshot = state(|s| {
        let tab = s.browser.active();
        (
            s.chrome.clone(),
            Snapshot {
                tabs: s
                    .browser
                    .tabs()
                    .iter()
                    .map(|t| (t.id(), t.source().title()))
                    .collect(),
                active: tab.id(),
                can_back: tab.can_back(),
                can_forward: tab.can_forward(),
                address: address.then(|| tab.source().address()),
            },
            s.dpi,
            format!("OpenSequoia — {}", tab.source().title()),
        )
    })
    .ok_or("native state unavailable")?;
    if let Some(controls) = snapshot.0 {
        let mut rect = RECT::default();
        // SAFETY: Own HWND and initialized output rectangle.
        if unsafe { GetClientRect(hwnd, &mut rect) } == 0 {
            return Err(os_error("control viewport failed"));
        }
        controls.refresh(&snapshot.1, rect.right, snapshot.2)?;
    }
    let title = wide(&snapshot.3);
    // SAFETY: Live terminated UTF-16 copied synchronously by own window.
    if unsafe { SetWindowTextW(hwnd, title.as_ptr()) } == 0 {
        return Err(os_error("title update failed"));
    }
    invalidate(hwnd)
}
fn queue(hwnd: HWND, request: LoadRequest) -> Result<(), String> {
    state(|s| {
        s.notice = None;
        if request.reason != LoadReason::Reflow {
            s.frames.remove(&request.key.tab);
            s.scrolls.insert(request.key.tab, 0);
        }
        if let Err(e) = s.controller.submit(request.clone(), s.width) {
            s.browser.complete(request.key, Err(e));
        }
    })
    .ok_or("navigation state unavailable")?;
    refresh(hwnd, request.reason != LoadReason::Reflow)
}
fn act(hwnd: HWND, action: Action) -> Result<(), String> {
    if action == Action::FocusAddress {
        if let Some(c) = state(|s| s.chrome.clone()).flatten() {
            c.focus_address();
        }
        return Ok(());
    }
    let source = match action {
        Action::Go => {
            let controls = state(|s| s.chrome.clone())
                .flatten()
                .ok_or("address control unavailable")?;
            let parsed = controls.read_address().and_then(|address| {
                PageSource::from_address(&address).map_err(|error| error.to_string())
            });
            match parsed {
                Ok(source) => Some(source),
                Err(_) => {
                    state(|s| {
                        s.notice = Some(
                            "Use about:welcome, about:guide or a trusted local HTML filename."
                                .to_owned(),
                        )
                    });
                    return refresh(hwnd, true);
                }
            }
        }
        Action::Guide => Some(PageSource::Guide),
        _ => None,
    };
    let request = state(|s| {
        let request = match action {
            Action::Back => s.browser.back().map(Some),
            Action::Forward => s.browser.forward().map(Some),
            Action::Reload => s.browser.reload().map(Some),
            Action::NewTab => s.browser.new_tab().map(Some),
            Action::Go | Action::Guide => s
                .browser
                .navigate(source.ok_or("navigation source unavailable")?)
                .map(Some),
            Action::CloseTab => {
                let id = s.browser.active().id();
                s.controller.cancel(id);
                s.frames.remove(&id);
                s.scrolls.remove(&id);
                s.browser.close(id)
            }
            Action::Select(id) => s.browser.select(id).map(|_| None),
            Action::NextTab => {
                s.browser.select_relative(1);
                Ok(None)
            }
            Action::PreviousTab => {
                s.browser.select_relative(-1);
                Ok(None)
            }
            Action::FocusAddress => Ok(None),
        };
        request.map_err(|e| e.to_string())
    })
    .ok_or("browser state unavailable")?;
    match request {
        Ok(Some(request)) => queue(hwnd, request),
        Ok(None) => {
            refresh(hwnd, true)?;
            rerender_if_needed(hwnd)
        }
        Err(e) => {
            state(|s| s.notice = Some(e));
            refresh(hwnd, false)
        }
    }
}
fn rerender_if_needed(hwnd: HWND) -> Result<(), String> {
    let request = state(reflow_request).ok_or("reflow state unavailable")??;
    if let Some(request) = request {
        queue(hwnd, request)
    } else {
        Ok(())
    }
}

fn reflow_request(s: &mut State) -> Result<Option<LoadRequest>, String> {
    let tab = s.browser.active();
    let needed = matches!(tab.status(), LoadStatus::Ready)
        && s.frames
            .get(&tab.id())
            .is_none_or(|frame| frame.width != s.width);
    if needed {
        s.browser.rerender().map(Some).map_err(|e| e.to_string())
    } else {
        Ok(None)
    }
}
fn rerender(hwnd: HWND) -> Result<(), String> {
    let request = state(|s| s.browser.rerender().map_err(|e| e.to_string()))
        .ok_or("reflow state unavailable")??;
    queue(hwnd, request)
}
fn poll(hwnd: HWND) -> Result<(), String> {
    let (changed, request) = state(|s| {
        let replies = s.controller.drain();
        accept_replies(s, replies)
    })
    .ok_or("render result state unavailable")??;
    if let Some(request) = request {
        queue(hwnd, request)?;
    } else if changed {
        refresh(hwnd, false)?;
    }
    Ok(())
}

/// Accept asynchronous replies and immediately emit a same-source reflow when
/// the foreground tab completed at an outdated width. Returning the request
/// keeps controller submission and every native call outside this state borrow.
fn accept_replies(
    s: &mut State,
    replies: Vec<RenderResult>,
) -> Result<(bool, Option<LoadRequest>), String> {
    let mut changed = false;
    for reply in replies {
        if !s.browser.is_current(reply.key) {
            continue;
        }
        let completion = reply.result.as_ref().map(|_| ()).map_err(Clone::clone);
        if s.browser.complete(reply.key, completion) {
            if let Ok(display) = reply.result {
                s.frames.insert(reply.key.tab, Rc::new(display));
            }
            clamp_scroll(s, reply.key.tab);
            changed = true;
        }
    }
    let reflow = if changed { reflow_request(s)? } else { None };
    Ok((changed, reflow))
}
fn resize(hwnd: HWND) -> Result<(), String> {
    let mut rect = RECT::default();
    // SAFETY: Own HWND and initialized output. DPI message LPARAM is not read.
    let dpi = unsafe {
        if GetClientRect(hwnd, &mut rect) == 0 {
            return Err(os_error("viewport query failed"));
        }
        GetDpiForWindow(hwnd)
    };
    if dpi == 0 {
        return Err(os_error("window DPI query failed"));
    }
    let width =
        (i64::from(rect.right.max(0)) * i64::from(CSS_DPI) / i64::from(dpi)).clamp(64, 8192) as u32;
    let full_height = i64::from(rect.bottom.max(0)) * i64::from(CSS_DPI) / i64::from(dpi);
    let height = u32::try_from((full_height - i64::from(chrome::TOP + chrome::BOTTOM)).max(0))
        .map_err(|_| "viewport height overflow")?;
    let reflow = state(|s| {
        let changed = s.width != width || s.dpi != dpi;
        s.width = width;
        s.height = height;
        s.dpi = dpi;
        let id = s.browser.active().id();
        clamp_scroll(s, id);
        let pending = changed && s.chrome.is_some();
        if pending {
            s.reflow_pending = true;
        }
        pending
    })
    .unwrap_or(false);
    refresh(hwnd, false)?;
    if reflow {
        // SAFETY: Own timer with bounded numeric delay; debounce repeated resize.
        if unsafe { SetTimer(hwnd, REFLOW_TIMER, 120, None) } == 0 {
            return Err(os_error("reflow timer failed"));
        }
    }
    Ok(())
}
fn clamp_scroll(s: &mut State, id: TabId) {
    let max = s
        .frames
        .get(&id)
        .map(|d| i32::try_from(d.height.saturating_sub(s.height)).unwrap_or(i32::MAX))
        .unwrap_or(0);
    let value = s.scrolls.entry(id).or_insert(0);
    *value = (*value).clamp(0, max);
}
fn scroll_by(hwnd: HWND, amount: i32) {
    let changed = state(|s| {
        let id = s.browser.active().id();
        let previous = *s.scrolls.get(&id).unwrap_or(&0);
        s.scrolls.insert(id, previous.saturating_add(amount));
        clamp_scroll(s, id);
        s.scrolls.get(&id).copied().unwrap_or(0) != previous
    })
    .unwrap_or(false);
    if changed && let Err(e) = invalidate(hwnd) {
        fail(hwnd, e);
    }
}
fn invalidate(hwnd: HWND) -> Result<(), String> {
    // SAFETY: Null rectangle invalidates own client area; no pointer read.
    if unsafe { InvalidateRect(hwnd, ptr::null(), 0) } == 0 {
        Err(os_error("window invalidation failed"))
    } else {
        Ok(())
    }
}
fn smoke_tick(hwnd: HWND) -> Result<(), String> {
    let next = state(|s| {
        let timeout = if matches!(s.smoke, Smoke::App { .. }) {
            Duration::from_secs(25)
        } else {
            Duration::from_secs(10)
        };
        if !matches!(s.smoke, Smoke::None) && s.started.elapsed() > timeout {
            return Err("native smoke timed out".to_owned());
        }
        let Smoke::App { stage, first } = s.smoke else {
            return Ok(None);
        };
        if !matches!(s.browser.active().status(), LoadStatus::Ready) || s.paints == 0 {
            return Ok(None);
        }
        let expected = match stage {
            1 | 3 | 6 | 8 => PageSource::Guide,
            _ => PageSource::Welcome,
        };
        if *s.browser.active().source() != expected {
            return Err(format!("native smoke source mismatch at stage {stage}"));
        }
        let frame = s
            .frames
            .get(&s.browser.active().id())
            .ok_or("native smoke frame missing")?;
        let expected_word = if expected == PageSource::Guide {
            "Browse"
        } else {
            "OpenSequoia"
        };
        if !frame.items.first().is_some_and(|item| match item {
            opensequoia_engine::DisplayItem::Text(run) => run.text == expected_word,
            opensequoia_engine::DisplayItem::Fill(_) => false,
        }) {
            return Err(format!("native smoke content mismatch at stage {stage}"));
        }
        if stage == 5 && s.browser.active().id() == first {
            return Err("new tab identity did not change".to_owned());
        }
        if stage == 9 && s.browser.tabs().len() != 1 {
            return Err("last-tab reset did not leave one tab".to_owned());
        }
        let action = match stage {
            0 => Action::Guide,
            1 => Action::Back,
            2 => Action::Forward,
            3 => Action::Back,
            4 => Action::NewTab,
            5 => Action::Guide,
            6 => Action::Select(first),
            7 => Action::CloseTab,
            8 => Action::CloseTab,
            9 => {
                s.smoke = Smoke::App { stage: 10, first };
                return Ok(Some(None));
            }
            _ => return Ok(None),
        };
        s.smoke = Smoke::App {
            stage: stage + 1,
            first,
        };
        s.paints = 0;
        Ok(Some(Some(action)))
    })
    .ok_or("smoke state unavailable")??;
    match next {
        Some(Some(action)) => {
            let controls = state(|s| s.chrome.clone())
                .flatten()
                .ok_or("smoke controls unavailable")?;
            if action == Action::Guide {
                // Exercise a rejected native Go before valid navigation. Its
                // provisional address must not remain over the committed page.
                let before =
                    state(|s| s.browser.active().request_key()).ok_or("smoke key unavailable")?;
                let committed = state(|s| s.browser.active().source().address())
                    .ok_or("smoke address unavailable")?;
                controls.set_address("https://example.invalid/")?;
                controls.invoke(Action::Go)?;
                let after =
                    state(|s| s.browser.active().request_key()).ok_or("smoke key unavailable")?;
                if before != after || controls.read_address()? != committed {
                    return Err("rejected native Go changed the committed page or retained its provisional address".to_owned());
                }
                controls.set_address("about:guide")?;
                controls.invoke(Action::Go)
            } else {
                controls.invoke(action)
            }
        }
        Some(None) => close_window(hwnd),
        None => Ok(()),
    }
}

/// Volume preflight is not sandboxing: concurrent drive mappings can still race.
pub(crate) fn reject_nonlocal_volume(path: &Path) -> Result<(), String> {
    let drive = match path.components().next() {
        Some(Component::Prefix(p)) => match p.kind() {
            Prefix::Disk(d) => d,
            _ => return Err("local input requires an ordinary disk path".to_owned()),
        },
        _ => return Err("local input requires an absolute disk path".to_owned()),
    };
    let root = wide(&format!("{}:\\", char::from(drive)));
    // SAFETY: Live terminated disk-root string, no content-supplied pointer.
    match unsafe { GetDriveTypeW(root.as_ptr()) } {
        2 | 3 | 5 | 6 => Ok(()),
        _ => Err("local input rejects network, absent or unknown volumes".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opensequoia_engine::Document;

    #[test]
    fn delayed_foreground_reply_after_other_tab_resize_emits_current_width_reflow() {
        let (browser, initial) = Browser::new(PageSource::Welcome).unwrap();
        let mut s = State {
            browser,
            controller: RenderController::new().unwrap(),
            initial: None,
            chrome: None,
            frames: BTreeMap::new(),
            scrolls: BTreeMap::new(),
            width: 960,
            height: 640,
            dpi: 96,
            reflow_pending: false,
            notice: None,
            failure: None,
            smoke: Smoke::None,
            started: Instant::now(),
            paints: 0,
            total_paints: 0,
            text_runs: 0,
        };
        let other = s.browser.new_tab().unwrap();
        s.width = 640;
        let document = Document::parse(include_str!("../../../fixtures/welcome.html")).unwrap();
        let (changed, reflow) = accept_replies(
            &mut s,
            vec![RenderResult {
                key: other.key,
                result: Ok(document.layout(640).unwrap()),
            }],
        )
        .unwrap();
        assert!(changed && reflow.is_none());
        s.browser.select(initial.key.tab).unwrap();
        assert!(matches!(s.browser.active().status(), LoadStatus::Loading));
        let (changed, reflow) = accept_replies(
            &mut s,
            vec![RenderResult {
                key: initial.key,
                result: Ok(document.layout(960).unwrap()),
            }],
        )
        .unwrap();
        assert!(changed);
        let reflow = reflow.expect("late foreground reply must schedule its current viewport");
        assert_eq!(reflow.key.tab, initial.key.tab);
        assert_eq!(reflow.reason, LoadReason::Reflow);
        assert!(reflow.key.generation > initial.key.generation);
        assert_eq!(s.browser.active().history_len(), 1);
        assert!(matches!(s.browser.active().status(), LoadStatus::Loading));
        assert_eq!(s.frames[&initial.key.tab].width, 960);
        let (changed, pending) = accept_replies(
            &mut s,
            vec![RenderResult {
                key: reflow.key,
                result: Ok(document.layout(640).unwrap()),
            }],
        )
        .unwrap();
        assert!(changed && pending.is_none());
        assert!(matches!(s.browser.active().status(), LoadStatus::Ready));
        assert_eq!(s.frames[&initial.key.tab].width, 640);
        let (changed, pending) = accept_replies(
            &mut s,
            vec![RenderResult {
                key: initial.key,
                result: Ok(document.layout(960).unwrap()),
            }],
        )
        .unwrap();
        assert!(!changed && pending.is_none());
        assert_eq!(s.frames[&initial.key.tab].width, 640);
    }
}

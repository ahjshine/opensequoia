//! Native Windows controls and GDI presentation. No browser engine is supplied by
//! these APIs. Every handle stays on the creating UI thread, and borrowed state
//! is released before calls that may reenter the window procedure.

use std::{cell::RefCell, ptr, rc::Rc};

use opensequoia_engine::{DisplayItem, DisplayList, TextRun};
use windows_sys::Win32::{
    Foundation::{HWND, RECT, SIZE},
    Graphics::Gdi::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Controls::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

use crate::browser::TabId;

pub const TOP: i32 = 84;
pub const BOTTOM: i32 = 24;
const CSS_DPI: u32 = 96;
const ADDRESS_LIMIT: usize = 4096;
const TAB_BASE: u16 = 100;
const TAB_COUNT: usize = 32;
// Documented Win32 STATIC style SS_CENTERIMAGE; generated windows-sys 0.61.2
// SystemServices/mod.rs:4350 declares 512. No extra feature is needed for a value.
const STATIC_CENTER_IMAGE: u32 = 0x200;

pub const BACK: u16 = 1;
pub const FORWARD: u16 = 2;
pub const RELOAD: u16 = 3;
pub const GO: u16 = 4;
pub const NEW_TAB: u16 = 5;
pub const CLOSE_TAB: u16 = 6;
pub const GUIDE: u16 = 7;
pub const FOCUS_ADDRESS: u16 = 8;
pub const NEXT_TAB: u16 = 9;
pub const PREVIOUS_TAB: u16 = 10;
const ADDRESS: u16 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Back,
    Forward,
    Reload,
    Go,
    NewTab,
    CloseTab,
    Guide,
    FocusAddress,
    NextTab,
    PreviousTab,
    Select(TabId),
}

pub struct Snapshot {
    pub tabs: Vec<(TabId, String)>,
    pub active: TabId,
    pub can_back: bool,
    pub can_forward: bool,
    pub address: Option<String>,
}

struct Font(HFONT);

impl Drop for Font {
    fn drop(&mut self) {
        // SAFETY: Font is owned, no DC selects it here. Native controls have
        // either been destroyed or switched to the replacement font first.
        unsafe { DeleteObject(self.0) };
    }
}

struct Children(Vec<HWND>);

impl Drop for Children {
    fn drop(&mut self) {
        // SAFETY: These are our controls on the UI thread. Parent teardown may
        // already have destroyed them; no other application handle is stored.
        for &child in &self.0 {
            unsafe {
                if IsWindow(child) != 0 {
                    DestroyWindow(child);
                }
            }
        }
    }
}

/// Owns a fixed set of controls. Tab buttons are reused, so repeated navigation
/// does not allocate an ever-growing collection of HWNDs. OS fonts are permitted
/// platform services; no font binary or native browser implementation is bundled.
pub struct Chrome {
    children: Children,
    back: HWND,
    forward: HWND,
    reload: HWND,
    go: HWND,
    new_tab: HWND,
    close_tab: HWND,
    guide: HWND,
    address_label: HWND,
    address: HWND,
    tabs: Vec<HWND>,
    mapping: RefCell<Vec<Option<TabId>>>,
    font: RefCell<(u32, Rc<Font>)>,
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn os_error(action: &str) -> String {
    format!("{action}: {}", std::io::Error::last_os_error())
}

fn ui_font(dpi: u32) -> Result<Font, String> {
    let face = wide("Segoe UI");
    let size = scaled(15, dpi)?.max(1);
    // SAFETY: All numeric fields are bounded, face is live terminated UTF-16.
    // Successful font is owned and is restored/deleted by its owner.
    let font = unsafe {
        CreateFontW(
            -size,
            0,
            0,
            0,
            FW_NORMAL as i32,
            0,
            0,
            0,
            u32::from(DEFAULT_CHARSET),
            u32::from(OUT_DEFAULT_PRECIS),
            u32::from(CLIP_DEFAULT_PRECIS),
            u32::from(CLEARTYPE_QUALITY),
            0,
            face.as_ptr(),
        )
    };
    if font.is_null() {
        Err(os_error("native UI font failed"))
    } else {
        Ok(Font(font))
    }
}

impl Chrome {
    pub fn create(parent: HWND, dpi: u32) -> Result<Self, String> {
        let font = Rc::new(ui_font(dpi)?);
        // Declared after font: partial creation destroys controls before font.
        let mut children = Children(Vec::new());
        let mut create = |class: &str, label: &str, id: u16, extra: u32| {
            let tab_stop = if class == "STATIC" { 0 } else { WS_TABSTOP };
            let class = wide(class);
            let label = wide(label);
            // SAFETY: Parent is our live window, IDs are values, no Rust pointer
            // is placed in HWND data/lParam. Win32 copies these strings.
            let child = unsafe {
                CreateWindowExW(
                    0,
                    class.as_ptr(),
                    label.as_ptr(),
                    WS_CHILD | WS_VISIBLE | tab_stop | extra,
                    0,
                    0,
                    1,
                    1,
                    parent,
                    usize::from(id) as HMENU,
                    GetModuleHandleW(ptr::null()),
                    ptr::null(),
                )
            };
            if child.is_null() {
                return Err(os_error("native control creation failed"));
            }
            children.0.push(child);
            // SAFETY: Created control/font remain alive; no borrowed Rust state.
            unsafe { SendMessageW(child, WM_SETFONT, font.0 as usize, 0) };
            Ok(child)
        };
        let back = create("BUTTON", "Back", BACK, BS_PUSHBUTTON as u32)?;
        let forward = create("BUTTON", "Forward", FORWARD, BS_PUSHBUTTON as u32)?;
        let reload = create("BUTTON", "Reload", RELOAD, BS_PUSHBUTTON as u32)?;
        let address_label = create("STATIC", "Address", 21, STATIC_CENTER_IMAGE)?;
        let address = create("EDIT", "", ADDRESS, ES_AUTOHSCROLL as u32 | WS_BORDER)?;
        let go = create("BUTTON", "Go", GO, BS_PUSHBUTTON as u32)?;
        let new_tab = create("BUTTON", "+ New tab", NEW_TAB, BS_PUSHBUTTON as u32)?;
        let close_tab = create("BUTTON", "Close tab", CLOSE_TAB, BS_PUSHBUTTON as u32)?;
        let guide = create("BUTTON", "Guide", GUIDE, BS_PUSHBUTTON as u32)?;
        let mut tabs = Vec::with_capacity(TAB_COUNT);
        for index in 0..TAB_COUNT {
            tabs.push(create(
                "BUTTON",
                "",
                TAB_BASE + index as u16,
                BS_PUSHBUTTON as u32,
            )?);
        }
        // SAFETY: Native edit accepts a numeric length limit, no pointer.
        unsafe { SendMessageW(address, EM_LIMITTEXT, ADDRESS_LIMIT, 0) };
        Ok(Self {
            children,
            back,
            forward,
            reload,
            go,
            new_tab,
            close_tab,
            guide,
            address_label,
            address,
            tabs,
            mapping: RefCell::new(vec![None; TAB_COUNT]),
            font: RefCell::new((dpi, font)),
        })
    }

    pub fn refresh(&self, view: &Snapshot, width: i32, dpi: u32) -> Result<(), String> {
        let font_changed = self.font.borrow().0 != dpi;
        if font_changed {
            let new_font = Rc::new(ui_font(dpi)?);
            // Old owner remains alive until all controls use the replacement.
            let old_font = Rc::clone(&self.font.borrow().1);
            for &child in &self.children.0 {
                // SAFETY: Owned child and new font are live, guards are released.
                unsafe { SendMessageW(child, WM_SETFONT, new_font.0 as usize, 1) };
            }
            *self.font.borrow_mut() = (dpi, new_font);
            drop(old_font);
        }
        let logical = (i64::from(width.max(0)) * i64::from(CSS_DPI) / i64::from(dpi.max(1))) as i32;
        let capacity = ((logical - 182).max(48) / 48).clamp(1, TAB_COUNT as i32) as usize;
        let active = view
            .tabs
            .iter()
            .position(|(id, _)| *id == view.active)
            .unwrap_or(0);
        let visible = capacity.min(view.tabs.len());
        let start = active.saturating_sub(visible.saturating_sub(1));
        let tab_width = ((logical - 182).max(48) / visible.max(1) as i32).clamp(24, 190);
        let mut mapping = vec![None; TAB_COUNT];
        for (index, (id, _)) in view.tabs.iter().enumerate().skip(start).take(visible) {
            mapping[index] = Some(*id);
        }
        *self.mapping.borrow_mut() = mapping;
        for (index, &button) in self.tabs.iter().enumerate() {
            let shown = index >= start && index < start + visible;
            if shown {
                let (id, label) = &view.tabs[index];
                let caption: String = label.chars().take(30).collect();
                set_text(button, &caption)?;
                place(
                    button,
                    8 + (index - start) as i32 * tab_width,
                    6,
                    tab_width - 3,
                    29,
                    dpi,
                )?;
                // SAFETY: Numeric push-button state, owned child; no pointers.
                unsafe { SendMessageW(button, BM_SETSTATE, usize::from(*id == view.active), 0) };
            }
            // SAFETY: These HWNDs are our fixed native controls on this thread.
            unsafe { ShowWindow(button, if shown { SW_SHOW } else { SW_HIDE }) };
        }
        place(self.new_tab, (logical - 168).max(58), 6, 84, 29, dpi)?;
        place(self.close_tab, (logical - 80).max(145), 6, 72, 29, dpi)?;
        let compact = logical < 700;
        place(self.back, 8, 44, 62, 30, dpi)?;
        place(self.forward, 74, 44, 70, 30, dpi)?;
        place(self.reload, 148, 44, 68, 30, dpi)?;
        let address_x = if compact { 74 } else { 222 };
        let guide_shown = logical >= 500;
        let right_buttons = if guide_shown { 116 } else { 52 };
        place(self.address_label, address_x, 44, 56, 30, dpi)?;
        place(
            self.address,
            address_x + 58,
            44,
            (logical - address_x - 58 - right_buttons - 8).max(40),
            30,
            dpi,
        )?;
        place(
            self.go,
            (logical - right_buttons).max(address_x + 102),
            44,
            44,
            30,
            dpi,
        )?;
        place(self.guide, (logical - 64).max(0), 44, 56, 30, dpi)?;
        // SAFETY: Enable/show only owned native controls. Return is prior state.
        unsafe {
            EnableWindow(self.back, i32::from(view.can_back));
            EnableWindow(self.forward, i32::from(view.can_forward));
            ShowWindow(self.forward, if compact { SW_HIDE } else { SW_SHOW });
            ShowWindow(self.reload, if compact { SW_HIDE } else { SW_SHOW });
            ShowWindow(self.guide, if guide_shown { SW_SHOW } else { SW_HIDE });
        }
        if let Some(address) = &view.address {
            set_text(self.address, address)?;
        }
        Ok(())
    }

    /// WM_COMMAND carries a numeric ID and HWND, never a dereferenced pointer.
    /// Notifications are accepted only from the matching owned control; the
    /// separately handled accelerator IDs use lParam=0 and notification=1.
    pub fn command(&self, id: u16, notification: u16, sender: HWND) -> Option<Action> {
        if notification != BN_CLICKED as u16 {
            return None;
        }
        let pairs = [
            (BACK, self.back, Action::Back),
            (FORWARD, self.forward, Action::Forward),
            (RELOAD, self.reload, Action::Reload),
            (GO, self.go, Action::Go),
            (NEW_TAB, self.new_tab, Action::NewTab),
            (CLOSE_TAB, self.close_tab, Action::CloseTab),
            (GUIDE, self.guide, Action::Guide),
        ];
        for (expected, handle, action) in pairs {
            if id == expected && sender == handle {
                return Some(action);
            }
        }
        let index = usize::from(id.checked_sub(TAB_BASE)?);
        if self.tabs.get(index).copied()? != sender {
            return None;
        }
        self.mapping
            .borrow()
            .get(index)
            .copied()
            .flatten()
            .map(Action::Select)
    }

    pub fn accelerator(id: u16) -> Option<Action> {
        match id {
            BACK => Some(Action::Back),
            FORWARD => Some(Action::Forward),
            RELOAD => Some(Action::Reload),
            NEW_TAB => Some(Action::NewTab),
            CLOSE_TAB => Some(Action::CloseTab),
            FOCUS_ADDRESS => Some(Action::FocusAddress),
            NEXT_TAB => Some(Action::NextTab),
            PREVIOUS_TAB => Some(Action::PreviousTab),
            _ => None,
        }
    }

    pub fn read_address(&self) -> Result<String, String> {
        // SAFETY: Query owned native edit. Length is checked before allocating;
        // an external WM_SETTEXT cannot force an unbounded buffer here.
        let length = unsafe { GetWindowTextLengthW(self.address) };
        if !(0..=ADDRESS_LIMIT as i32).contains(&length) {
            return Err("address is too long".to_owned());
        }
        let mut text = vec![0u16; length as usize + 1];
        // SAFETY: Live initialized buffer, count includes terminator and is bounded.
        let read = unsafe { GetWindowTextW(self.address, text.as_mut_ptr(), text.len() as i32) };
        String::from_utf16(&text[..read as usize])
            .map_err(|_| "address must contain valid Unicode".to_owned())
    }

    pub fn set_address(&self, address: &str) -> Result<(), String> {
        set_text(self.address, address)
    }

    pub fn focus_address(&self) {
        // SAFETY: Live owned edit, numeric selection command; no state borrow.
        unsafe {
            SetFocus(self.address);
            SendMessageW(self.address, EM_SETSEL, 0, -1);
        }
    }

    pub fn editor_focused(&self) -> bool {
        // SAFETY: Read thread-local OS focus, not a pointer supplied by content.
        unsafe { GetFocus() == self.address }
    }

    pub fn click_focused(&self) -> bool {
        // SAFETY: Read focus and click only our own known button HWND.
        let focused = unsafe { GetFocus() };
        let navigation = [
            self.back,
            self.forward,
            self.reload,
            self.go,
            self.new_tab,
            self.close_tab,
            self.guide,
        ];
        if navigation.contains(&focused) || self.tabs.contains(&focused) {
            unsafe { SendMessageW(focused, BM_CLICK, 0, 0) };
            true
        } else {
            false
        }
    }

    /// Internal native smoke probe: operate the same BUTTON message path as
    /// Windows accessibility invocation, then verify the asynchronous outcome.
    pub fn invoke(&self, action: Action) -> Result<(), String> {
        let handle = match action {
            Action::Back => self.back,
            Action::Forward => self.forward,
            Action::Reload => self.reload,
            Action::Go => self.go,
            Action::NewTab => self.new_tab,
            Action::CloseTab => self.close_tab,
            Action::Guide => self.guide,
            Action::Select(id) => {
                let index = self
                    .mapping
                    .borrow()
                    .iter()
                    .position(|mapped| *mapped == Some(id))
                    .ok_or("smoke tab button is not visible")?;
                self.tabs[index]
            }
            _ => return Err("action has no native button".to_owned()),
        };
        // SAFETY: Known owned live button; no mapping/state borrow crosses the
        // synchronous call, which invokes our parent WM_COMMAND handler.
        unsafe { SendMessageW(handle, BM_CLICK, 0, 0) };
        Ok(())
    }
}

fn set_text(hwnd: HWND, value: &str) -> Result<(), String> {
    if value.contains('\0') || value.len() > ADDRESS_LIMIT {
        return Err("native label is invalid".to_owned());
    }
    let value = wide(value);
    // SAFETY: Live terminated UTF-16 copied synchronously by owned control.
    if unsafe { SetWindowTextW(hwnd, value.as_ptr()) } == 0 {
        Err(os_error("native text update failed"))
    } else {
        Ok(())
    }
}

fn place(hwnd: HWND, x: i32, y: i32, width: i32, height: i32, dpi: u32) -> Result<(), String> {
    // SAFETY: Own HWND, bounded checked pixel coordinates, no borrow or pointer payload.
    if unsafe {
        MoveWindow(
            hwnd,
            scaled(x, dpi)?,
            scaled(y, dpi)?,
            scaled(width.max(1), dpi)?.max(1),
            scaled(height, dpi)?.max(1),
            1,
        )
    } == 0
    {
        Err(os_error("native control positioning failed"))
    } else {
        Ok(())
    }
}

pub fn accelerators() -> Result<HACCEL, String> {
    let entries = [
        (FCONTROL, VK_L, FOCUS_ADDRESS),
        (FCONTROL, VK_T, NEW_TAB),
        (FCONTROL, VK_W, CLOSE_TAB),
        (FCONTROL, VK_TAB, NEXT_TAB),
        (FCONTROL | FSHIFT, VK_TAB, PREVIOUS_TAB),
        (FALT, VK_LEFT, BACK),
        (FALT, VK_RIGHT, FORWARD),
        (0, VK_F5, RELOAD),
    ]
    .map(|(flags, key, cmd)| ACCEL {
        fVirt: FVIRTKEY | flags,
        key,
        cmd,
    });
    // SAFETY: Initialized entries have static value IDs and live through OS copy.
    let table = unsafe { CreateAcceleratorTableW(entries.as_ptr(), entries.len() as i32) };
    if table.is_null() {
        Err(os_error("keyboard accelerator creation failed"))
    } else {
        Ok(table)
    }
}

struct Paint {
    hwnd: HWND,
    dc: HDC,
    info: PAINTSTRUCT,
    finished: bool,
}

impl Paint {
    fn begin(hwnd: HWND) -> Result<Self, String> {
        let mut info = PAINTSTRUCT::default();
        // SAFETY: Called only during WM_PAINT with initialized writable PAINTSTRUCT.
        let dc = unsafe { BeginPaint(hwnd, &mut info) };
        if dc.is_null() {
            let error = os_error("BeginPaint failed");
            // SAFETY: Pair the attempted BeginPaint even on its error path.
            unsafe { EndPaint(hwnd, &info) };
            return Err(error);
        }
        Ok(Self {
            hwnd,
            dc,
            info,
            finished: false,
        })
    }

    fn finish(&mut self) -> Result<(), String> {
        // SAFETY: Matches this Paint's BeginPaint, with the original struct and HWND.
        let success = unsafe { EndPaint(self.hwnd, &self.info) } != 0;
        self.finished = true;
        if success {
            Ok(())
        } else {
            Err(os_error("EndPaint failed"))
        }
    }
}

impl Drop for Paint {
    fn drop(&mut self) {
        if !self.finished {
            // SAFETY: Panic/error cleanup pairs the live BeginPaint. Failure cannot
            // establish smoke success; explicit normal finish is checked above.
            unsafe { EndPaint(self.hwnd, &self.info) };
        }
    }
}

pub fn scaled(value: i32, dpi: u32) -> Result<i32, String> {
    scaled_offset(i64::from(value), dpi)
}

/// Scale a signed CSS offset without clamping it at zero, so content scrolled
/// above the viewport keeps its true overhang and is clipped by GDI.
fn scaled_offset(value: i64, dpi: u32) -> Result<i32, String> {
    let scaled = value
        .checked_mul(i64::from(dpi))
        .ok_or_else(|| "drawing coordinate overflow".to_owned())?
        / i64::from(CSS_DPI);
    i32::try_from(scaled).map_err(|_| "drawing coordinate overflow".to_owned())
}

pub fn paint(
    hwnd: HWND,
    display: Option<&DisplayList>,
    scroll: i32,
    dpi: u32,
    status: &str,
) -> Result<u32, String> {
    let mut paint = Paint::begin(hwnd)?;
    let result = draw(paint.dc, hwnd, display, scroll, dpi, status);
    let finish = paint.finish();
    finish?;
    result
}

fn fill(dc: HDC, rect: &RECT, color: u32) -> Result<(), String> {
    // SAFETY: Own live paint DC and initialized rectangle; created brush is never selected.
    unsafe {
        let brush = CreateSolidBrush(color);
        if brush.is_null() {
            return Err(os_error("brush creation failed"));
        }
        let painted = FillRect(dc, rect, brush) != 0;
        let deleted = DeleteObject(brush) != 0;
        if !painted || !deleted {
            Err("background paint/cleanup failed".to_owned())
        } else {
            Ok(())
        }
    }
}

fn label(dc: HDC, value: &str, rect: &mut RECT, dpi: u32, single: bool) -> Result<(), String> {
    let font = ui_font(dpi)?;
    let text: Vec<u16> = value.encode_utf16().collect();
    // SAFETY: Own DC/font and initialized live buffers, exact bounded text count;
    // restore before owned Font drops. DrawText copies/reads without modification.
    unsafe {
        let previous = SelectObject(dc, font.0);
        if previous.is_null() || previous as isize == -1 {
            return Err(os_error("label font selection failed"));
        }
        let flags = DT_LEFT
            | DT_NOPREFIX
            | if single {
                DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS
            } else {
                DT_WORDBREAK
            };
        let result = if SetTextColor(dc, 0x002b4737) == CLR_INVALID
            || DrawTextW(dc, text.as_ptr(), text.len() as i32, rect, flags) == 0
        {
            Err(os_error("native label paint failed"))
        } else {
            Ok(())
        };
        let restored = SelectObject(dc, previous);
        if restored.is_null() || restored as isize == -1 {
            return Err("label font restoration failed".to_owned());
        }
        result
    }
}

fn draw(
    dc: HDC,
    hwnd: HWND,
    display: Option<&DisplayList>,
    scroll: i32,
    dpi: u32,
    status: &str,
) -> Result<u32, String> {
    let mut rect = RECT::default();
    // SAFETY: Own active paint DC/window and initialized output rectangle.
    unsafe {
        if GetClientRect(hwnd, &mut rect) == 0 {
            return Err(os_error("paint client rectangle failed"));
        }
        if SetBkMode(dc, TRANSPARENT as i32) == 0 {
            return Err(os_error("text background mode failed"));
        }
    }
    fill(dc, &rect, 0x00ffffff)?;
    let top = scaled(TOP, dpi)?.min(rect.bottom);
    let bottom = rect.bottom.saturating_sub(scaled(BOTTOM, dpi)?).max(top);
    let header = RECT {
        left: 0,
        top: 0,
        right: rect.right,
        bottom: top,
    };
    fill(dc, &header, 0x00edf4ef)?;
    let divider = RECT {
        left: 0,
        top: top.saturating_sub(2),
        right: rect.right,
        bottom: top,
    };
    fill(dc, &divider, 0x00527960)?;
    let footer = RECT {
        left: 0,
        top: bottom,
        right: rect.right,
        bottom: rect.bottom,
    };
    fill(dc, &footer, 0x00f1f5f2)?;
    if rect.right > 16 && footer.bottom > footer.top {
        let mut footer_text = RECT {
            left: scaled(10, dpi)?,
            top: bottom,
            right: rect.right - 8,
            bottom: rect.bottom,
        };
        label(dc, status, &mut footer_text, dpi, true)?;
    }
    let Some(display) = display else {
        if rect.right > 40 && bottom - top > 40 {
            let mut message = RECT {
                left: scaled(20, dpi)?,
                top: top + scaled(24, dpi)?,
                right: rect.right - scaled(20, dpi)?,
                bottom,
            };
            label(dc, status, &mut message, dpi, false)?;
        }
        return Ok(0);
    };
    // SAFETY: Own live DC; saved clipping state is restored on every normal path.
    let saved = unsafe { SaveDC(dc) };
    if saved == 0 {
        return Err(os_error("content clipping save failed"));
    }
    let result = (|| {
        if unsafe { IntersectClipRect(dc, 0, top, rect.right, bottom) } == ERROR {
            return Err(os_error("content clipping failed"));
        }
        let mut count = 0u32;
        for item in &display.items {
            match item {
                DisplayItem::Fill(rect) => {
                    let x = scaled(rect.x, dpi)?;
                    let y = scaled_offset(i64::from(rect.y) - i64::from(scroll), dpi)?
                        .saturating_add(top);
                    let width = scaled(
                        i32::try_from(rect.width).map_err(|_| "fill width overflow")?,
                        dpi,
                    )?;
                    let height = scaled(
                        i32::try_from(rect.height).map_err(|_| "fill height overflow")?,
                        dpi,
                    )?;
                    if y >= bottom || y.saturating_add(height) <= top {
                        continue;
                    }
                    let color = u32::from(rect.color.r)
                        | (u32::from(rect.color.g) << 8)
                        | (u32::from(rect.color.b) << 16);
                    let rect = RECT {
                        left: x,
                        top: y,
                        right: x.saturating_add(width),
                        bottom: y.saturating_add(height),
                    };
                    fill(dc, &rect, color)?;
                }
                DisplayItem::Text(run) => {
                    let y = scaled_offset(i64::from(run.y) - i64::from(scroll), dpi)?
                        .saturating_add(top);
                    let height = scaled(
                        i32::try_from(run.height).map_err(|_| "text height overflow")?,
                        dpi,
                    )?;
                    if y >= bottom || y.saturating_add(height) <= top {
                        continue;
                    }
                    draw_text(dc, run, y, dpi)?;
                    count = count.saturating_add(1);
                }
            }
        }
        Ok(count)
    })();
    if unsafe { RestoreDC(dc, saved) } == 0 {
        return Err(os_error("content clipping restore failed"));
    }
    result
}

fn draw_text(dc: HDC, run: &TextRun, y: i32, dpi: u32) -> Result<(), String> {
    let text: Vec<u16> = run.text.encode_utf16().collect();
    let count = i32::try_from(text.len()).map_err(|_| "text length overflow")?;
    if count == 0 {
        return Ok(());
    }
    let x = scaled(run.x, dpi)?;
    let color =
        u32::from(run.color.r) | (u32::from(run.color.g) << 8) | (u32::from(run.color.b) << 16);
    // SAFETY: DC is valid for this paint; text buffer remains live and TextOut
    // receives its exact count. font_for checks its own creation invariants.
    // Created font is restored out of the DC before deletion on every normal path.
    unsafe {
        let font = font_for(run, dpi)?;
        let previous = SelectObject(dc, font);
        if previous.is_null() || previous as isize == -1 {
            let error = os_error("font selection failed");
            return if DeleteObject(font) != 0 {
                Err(error)
            } else {
                Err(format!("{error}; font cleanup failed"))
            };
        }
        let result = ascii_width_fits(dc, run, &text, dpi).and_then(|()| {
            if SetTextColor(dc, color) == CLR_INVALID {
                Err(os_error("text color failed"))
            } else if TextOutW(dc, x, y, text.as_ptr(), count) == 0 {
                Err(os_error("TextOutW failed"))
            } else {
                Ok(())
            }
        });
        let restored = SelectObject(dc, previous);
        let deleted = DeleteObject(font) != 0;
        if restored.is_null() || restored as isize == -1 || !deleted {
            return Err("GDI font restoration/deletion failed".to_owned());
        }
        result
    }
}

/// Return an owned OS font. Callers must restore it out of every DC before
/// DeleteObject, and must never delete an OS stock object in its place.
fn font_for(run: &TextRun, dpi: u32) -> Result<HFONT, String> {
    let face = wide("Consolas");
    let size = scaled(
        i32::try_from(run.font_size).map_err(|_| "font size overflow")?,
        dpi,
    )?;
    if !(1..=512).contains(&size) {
        return Err("font size outside drawing limit".to_owned());
    }
    // Match this deliberately static engine's fixed ASCII cells. This does not
    // implement CSS font metrics, fallback shaping, bidi or grapheme advances.
    let cell = (run
        .font_size
        .checked_mul(3)
        .ok_or("font advance overflow")?
        / 5)
    .max(1);
    let width = scaled(
        i32::try_from(cell).map_err(|_| "font advance overflow")?,
        dpi,
    )?
    .max(1);
    // SAFETY: face is live terminated UTF-16, numeric fields are bounded and
    // documented CreateFontW parameters. A successful handle is caller-owned.
    let font = unsafe {
        CreateFontW(
            -size,
            width,
            0,
            0,
            if run.bold { FW_BOLD } else { FW_NORMAL } as i32,
            0,
            0,
            0,
            u32::from(DEFAULT_CHARSET),
            u32::from(OUT_DEFAULT_PRECIS),
            u32::from(CLIP_DEFAULT_PRECIS),
            u32::from(CLEARTYPE_QUALITY),
            u32::from(FIXED_PITCH | FF_MODERN),
            face.as_ptr(),
        )
    };
    if font.is_null() {
        Err(os_error("CreateFontW failed"))
    } else {
        Ok(font)
    }
}

fn ascii_width_fits(dc: HDC, run: &TextRun, text: &[u16], dpi: u32) -> Result<(), String> {
    if !run.text.is_ascii() {
        return Ok(()); // Complex Unicode shaping is explicitly unsupported.
    }
    let mut extent = SIZE::default();
    let count = i32::try_from(text.len()).map_err(|_| "font metric text length overflow")?;
    let permitted = scaled(
        i32::try_from(run.width).map_err(|_| "text run width overflow")?,
        dpi,
    )?;
    // SAFETY: Active DC has the owned font selected. Exact text length and a
    // live initialized writable SIZE are supplied; no borrow crosses a callback.
    if unsafe { GetTextExtentPoint32W(dc, text.as_ptr(), count, &mut extent) } == 0 {
        return Err(os_error("ASCII font metric query failed"));
    }
    if extent.cx > permitted {
        Err("OS ASCII font metrics exceed the static engine allocation".to_owned())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpi_conversion_checks_signed_coordinates() {
        assert_eq!(scaled(24, 144).unwrap(), 36);
        assert_eq!(scaled(-24, 192).unwrap(), -48);
        assert_eq!(scaled_offset(-24, 192).unwrap(), -48);
        assert_eq!(scaled_offset(-24 - 120, 192).unwrap(), -288);
        assert!(scaled(i32::MAX, 192).is_err());
        assert!(scaled_offset(i64::MAX, 192).is_err());
    }

    #[test]
    fn actual_ascii_font_metrics_fit_static_engine_advances() {
        let text = "WWWW MMMM 012345 DOMand";
        for dpi in [96, 120, 144, 192] {
            for size in [16, 18, 24, 32] {
                for bold in [false, true] {
                    let run = TextRun {
                        text: text.to_owned(),
                        x: 0,
                        y: 0,
                        width: text.len() as u32 * (size * 3 / 5).max(1),
                        height: size + 8,
                        font_size: size,
                        bold,
                        color: opensequoia_engine::Color { r: 0, g: 0, b: 0 },
                    };
                    let font = font_for(&run, dpi).unwrap();
                    let text: Vec<u16> = text.encode_utf16().collect();
                    let mut extent = SIZE::default();
                    // SAFETY: Test owns the memory DC/font and live initialized buffers.
                    // It restores/deletes font and DC before any assertions can panic.
                    let (measured, selected, restored, deleted_font, deleted_dc) = unsafe {
                        let dc = CreateCompatibleDC(ptr::null_mut());
                        if dc.is_null() {
                            DeleteObject(font);
                            panic!("test memory DC failed");
                        }
                        let previous = SelectObject(dc, font);
                        let selected = !previous.is_null() && previous as isize != -1;
                        let measured = selected
                            && GetTextExtentPoint32W(
                                dc,
                                text.as_ptr(),
                                text.len() as i32,
                                &mut extent,
                            ) != 0;
                        let restored = if selected {
                            let old = SelectObject(dc, previous);
                            !old.is_null() && old as isize != -1
                        } else {
                            true
                        };
                        let deleted_font = DeleteObject(font) != 0;
                        let deleted_dc = DeleteDC(dc) != 0;
                        (measured, selected, restored, deleted_font, deleted_dc)
                    };
                    assert!(selected && measured && restored && deleted_font && deleted_dc);
                    assert!(
                        extent.cx <= scaled(run.width as i32, dpi).unwrap(),
                        "OS advance {} exceeds static allocation {} size={size} bold={bold} dpi={dpi}",
                        extent.cx,
                        scaled(run.width as i32, dpi).unwrap()
                    );
                }
            }
        }
    }
}

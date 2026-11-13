//! A real AppKit NSPanel for the dictation pill.
//!
//! A winit/egui window cannot safely become a non-activating `NSPanel`, so this
//! module owns a small native panel that joins every Space, including
//! fullscreen apps, and never takes focus.

use std::ffi::{c_char, c_void, CString};
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Mutex;

type Id = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct NSRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct NSPoint {
    pub x: f64,
    pub y: f64,
}

#[link(name = "AppKit", kind = "framework")]
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn objc_getClass(name: *const c_char) -> Id;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
    fn objc_allocateClassPair(superclass: Id, name: *const c_char, extra: usize) -> Id;
    fn objc_registerClassPair(class: Id);
    fn class_addMethod(class: Id, sel: *mut c_void, imp: *const c_void, types: *const c_char)
        -> bool;
    fn NSRectFill(rect: NSRect);
    fn CFStringCreateWithBytes(
        alloc: *const c_void,
        bytes: *const u8,
        len: isize,
        encoding: u32,
        external: bool,
    ) -> Id;
    static NSFontAttributeName: *const c_void;
    static NSForegroundColorAttributeName: *const c_void;
    fn CFRelease(cf: *mut c_void);
}

#[derive(Clone, Default)]
pub struct Model {
    /// 0 hidden, 1 listening, 2 transcribing, 3 done-pasted, 4 done-copy
    pub kind: u8,
    pub elapsed: f32,
    pub bands: [f32; 7],
    pub label: String,
    pub text: String,
    pub copied: bool,
    /// Custom card heading (command mode); empty uses the default wording.
    pub head: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Stop,
    Copy,
    Dismiss,
}

static MODEL: Mutex<Model> = Mutex::new(Model {
    kind: 0,
    elapsed: 0.0,
    bands: [0.0; 7],
    label: String::new(),
    text: String::new(),
    copied: false,
    head: String::new(),
});
static HIT_RECTS: Mutex<Vec<(u8, NSRect)>> = Mutex::new(Vec::new());
static ACTIONS: Mutex<Vec<Action>> = Mutex::new(Vec::new());
static PANEL: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
static VIEW: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

pub const WIDTH: f64 = 470.0;
pub const HEIGHT: f64 = 176.0;

unsafe fn sel(name: &str) -> *mut c_void {
    let c = CString::new(name).unwrap();
    sel_registerName(c.as_ptr())
}

unsafe fn msg0(obj: Id, name: &str) -> Id {
    let f: extern "C" fn(Id, *mut c_void) -> Id = std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel(name))
}

unsafe fn msg1(obj: Id, name: &str, a: Id) -> Id {
    let f: extern "C" fn(Id, *mut c_void, Id) -> Id = std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel(name), a)
}

unsafe fn msg2(obj: Id, name: &str, a: Id, b: Id) -> Id {
    let f: extern "C" fn(Id, *mut c_void, Id, Id) -> Id =
        std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel(name), a, b)
}

unsafe fn msg_bool(obj: Id, name: &str, a: bool) {
    let f: extern "C" fn(Id, *mut c_void, bool) = std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel(name), a);
}

unsafe fn msg_i64(obj: Id, name: &str, a: i64) {
    let f: extern "C" fn(Id, *mut c_void, i64) = std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel(name), a);
}

unsafe fn msg_usize(obj: Id, name: &str, a: usize) {
    let f: extern "C" fn(Id, *mut c_void, usize) = std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel(name), a);
}

unsafe fn msg_rect(obj: Id, name: &str, r: NSRect) -> Id {
    let f: extern "C" fn(Id, *mut c_void, NSRect) -> Id =
        std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel(name), r)
}

unsafe fn nsstring(s: &str) -> Id {
    CFStringCreateWithBytes(
        std::ptr::null(),
        s.as_ptr(),
        s.len() as isize,
        0x0800_0100,
        false,
    )
}

unsafe fn color(r: f64, g: f64, b: f64, a: f64) -> Id {
    let f: extern "C" fn(Id, *mut c_void, f64, f64, f64, f64) -> Id =
        std::mem::transmute(objc_msgSend as *const ());
    f(
        objc_getClass(c"NSColor".as_ptr()),
        sel("colorWithSRGBRed:green:blue:alpha:"),
        r,
        g,
        b,
        a,
    )
}

unsafe fn fill_round(rect: NSRect, radius: f64, c: Id) {
    msg0(c, "setFill");
    let f: extern "C" fn(Id, *mut c_void, NSRect, f64, f64) -> Id =
        std::mem::transmute(objc_msgSend as *const ());
    let path = f(
        objc_getClass(c"NSBezierPath".as_ptr()),
        sel("bezierPathWithRoundedRect:xRadius:yRadius:"),
        rect,
        radius,
        radius,
    );
    msg0(path, "fill");
}

unsafe fn draw_text(s: &str, x: f64, y: f64, size: f64, c: Id) {
    let font: Id = {
        let f: extern "C" fn(Id, *mut c_void, f64) -> Id =
            std::mem::transmute(objc_msgSend as *const ());
        f(objc_getClass(c"NSFont".as_ptr()), sel("systemFontOfSize:"), size)
    };
    let attrs = msg0(msg0(objc_getClass(c"NSMutableDictionary".as_ptr()), "alloc"), "init");
    msg2(attrs, "setObject:forKey:", font, NSFontAttributeName as Id);
    msg2(attrs, "setObject:forKey:", c, NSForegroundColorAttributeName as Id);
    let text = nsstring(s);
    let f: extern "C" fn(Id, *mut c_void, NSPoint, Id) =
        std::mem::transmute(objc_msgSend as *const ());
    f(text, sel("drawAtPoint:withAttributes:"), NSPoint { x, y }, attrs);
    CFRelease(text);
    msg0(attrs, "release");
}

unsafe fn stroke_line(x1: f64, y1: f64, x2: f64, y2: f64, width: f64, c: Id) {
    msg0(c, "setStroke");
    let f: extern "C" fn(Id, *mut c_void, NSPoint) = std::mem::transmute(objc_msgSend as *const ());
    let path = msg0(objc_getClass(c"NSBezierPath".as_ptr()), "bezierPath");
    f(path, sel("moveToPoint:"), NSPoint { x: x1, y: y1 });
    f(path, sel("lineToPoint:"), NSPoint { x: x2, y: y2 });
    let w: extern "C" fn(Id, *mut c_void, f64) =
        std::mem::transmute(objc_msgSend as *const ());
    w(path, sel("setLineWidth:"), width);
    msg0(path, "stroke");
}

extern "C" fn draw_rect(_this: Id, _cmd: *mut c_void, _rect: NSRect) {
    let model = MODEL.lock().map(|m| m.clone()).unwrap_or_default();
    if model.kind == 0 {
        return;
    }
    let w = WIDTH;
    let h = HEIGHT;
    let white = unsafe { color(0.98, 0.98, 0.97, 1.0) };
    let muted = unsafe { color(0.64, 0.64, 0.62, 1.0) };
    let dark = unsafe { color(0.10, 0.10, 0.10, 1.0) };
    let glow = unsafe { color(1.0, 0.66, 0.27, 1.0) };
    let teal = unsafe { color(0.41, 0.74, 0.69, 1.0) };
    let mut hits = Vec::new();

    if model.kind == 4 {
        // Persistent transcript card when nothing was pasted.
        let card = NSRect { x: (w - 436.0) / 2.0, y: h - 78.0 - 52.0, w: 436.0, h: 104.0 };
        unsafe {
            fill_round(card, 18.0, dark);
            let head = if !model.head.is_empty() {
                model.head.as_str()
            } else if model.copied {
                "No text field focused · copied"
            } else {
                "No text field focused"
            };
            draw_text(head, card.x + 18.0, card.y + 14.0, 11.5, muted);
            let preview: String = if model.text.chars().count() > 84 {
                format!("{}…", model.text.chars().take(83).collect::<String>())
            } else {
                model.text.clone()
            };
            draw_text(&preview, card.x + 18.0, card.y + 36.0, 12.0, white);
            // Copy button
            let copy = NSRect { x: card.x + card.w - 108.0, y: card.y + card.h - 44.0, w: 84.0, h: 30.0 };
            msg0(color(0.19, 0.19, 0.18, 1.0), "setFill");
            let f: extern "C" fn(Id, *mut c_void, NSRect, f64, f64) -> Id =
                std::mem::transmute(objc_msgSend as *const ());
            let path = f(
                objc_getClass(c"NSBezierPath".as_ptr()),
                sel("bezierPathWithRoundedRect:xRadius:yRadius:"),
                copy,
                8.0,
                8.0,
            );
            msg0(path, "fill");
            draw_text(
                if model.copied { "Copied" } else { "Copy" },
                copy.x + 24.0,
                copy.y + 8.0,
                11.0,
                white,
            );
            draw_text("×", card.x + card.w - 24.0, card.y + 8.0, 15.0, muted);
            hits.push((2u8, copy));
            hits.push((
                3u8,
                NSRect { x: card.x + card.w - 32.0, y: card.y, w: 32.0, h: 32.0 },
            ));
        }
    } else {
        let pill = NSRect { x: (w - 306.0) / 2.0, y: h - 58.0 - 33.0, w: 306.0, h: 66.0 };
        unsafe {
            fill_round(pill, 22.0, dark);
            if model.kind == 1 || model.kind == 2 {
                for i in 0..7 {
                    let v = model.bands[i].clamp(0.0, 1.0);
                    let bh = 2.0_f64 + 13.0 * v as f64;
                    let bx = pill.x + 18.0 + i as f64 * 8.0;
                    let c = if v > 0.72 { glow } else { white };
                    msg0(c, "setFill");
                    NSRectFill(NSRect { x: bx, y: pill.y + 33.0 - bh, w: 3.0, h: bh * 2.0 });
                }
                let title = if model.kind == 1 { "Listening" } else { "Transcribing" };
                draw_text(title, pill.x + 90.0, pill.y + 15.0, 13.0, white);
                let sub = format!("{} · {:.1} s", model.label, model.elapsed);
                draw_text(&sub, pill.x + 90.0, pill.y + 36.0, 10.5, muted);
                let stop = NSRect { x: pill.x + pill.w - 44.0, y: pill.y + 18.0, w: 30.0, h: 30.0 };
                msg0(color(0.19, 0.19, 0.18, 1.0), "setFill");
                let f: extern "C" fn(Id, *mut c_void, NSRect, f64, f64) -> Id =
                    std::mem::transmute(objc_msgSend as *const ());
                let path = f(
                    objc_getClass(c"NSBezierPath".as_ptr()),
                    sel("bezierPathWithRoundedRect:xRadius:yRadius:"),
                    stop,
                    8.0,
                    8.0,
                );
                msg0(path, "fill");
                msg0(white, "setFill");
                NSRectFill(NSRect { x: stop.x + 11.0, y: stop.y + 11.0, w: 8.0, h: 8.0 });
                hits.push((1u8, stop));
            } else {
                // Done, pasted
                msg0(teal, "setFill");
                NSRectFill(NSRect { x: pill.x + 33.0, y: pill.y + 28.0, w: 10.0, h: 10.0 });
                draw_text("Pasted", pill.x + 90.0, pill.y + 15.0, 13.0, white);
                let sub = format!("{} · {:.1} s", model.label, model.elapsed);
                draw_text(&sub, pill.x + 90.0, pill.y + 36.0, 10.5, muted);
                hits.push((3u8, pill));
            }
        }
    }

    if let Ok(mut r) = HIT_RECTS.lock() {
        *r = hits;
    }
}

extern "C" fn mouse_down(this: Id, _cmd: *mut c_void, event: Id) {
    unsafe {
        let loc: NSPoint = {
            let f: extern "C" fn(Id, *mut c_void) -> NSPoint =
                std::mem::transmute(objc_msgSend as *const ());
            f(event, sel("locationInWindow"))
        };
        let conv: extern "C" fn(Id, *mut c_void, NSPoint, Id) -> NSPoint =
            std::mem::transmute(objc_msgSend as *const ());
        let p = conv(this, sel("convertPoint:fromView:"), loc, std::ptr::null_mut());
        if std::env::var_os("FLOWLINE_DEBUG").is_some() {
            eprintln!("overlay mouse {:.0},{:.0}", p.x, p.y);
        }
        let mut action = None;
        if let Ok(hits) = HIT_RECTS.lock() {
            if std::env::var_os("FLOWLINE_DEBUG").is_some() {
                eprintln!("overlay hits {:?}", hits.len());
            }
            for (k, r) in hits.iter() {
                if p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h {
                    action = match k {
                        1 => Some(Action::Stop),
                        2 => Some(Action::Copy),
                        3 => Some(Action::Dismiss),
                        _ => None,
                    };
                    break;
                }
            }
        }
        if std::env::var_os("FLOWLINE_DEBUG").is_some() {
            eprintln!("overlay action {:?}", action.map(|a| a as u8));
        }
        if let Some(a) = action {
            if let Ok(mut q) = ACTIONS.lock() {
                q.push(a);
            }
        } else {
            let window = msg0(this, "window");
            let f: extern "C" fn(Id, *mut c_void, Id) =
                std::mem::transmute(objc_msgSend as *const ());
            f(window, sel("performWindowDragWithEvent:"), event);
        }
    }
}

extern "C" fn is_flipped(_this: Id, _cmd: *mut c_void) -> bool {
    true
}

extern "C" fn accepts_first_mouse(_this: Id, _cmd: *mut c_void, _event: Id) -> bool {
    true
}

pub fn install() {
    if !PANEL.load(Ordering::SeqCst).is_null() {
        return;
    }
    unsafe {
        let cls = objc_allocateClassPair(
            objc_getClass(c"NSView".as_ptr()),
            c"FlowlineOverlayView".as_ptr(),
            0,
        );
        class_addMethod(
            cls,
            sel("drawRect:"),
            draw_rect as *const c_void,
            c"v@:{CGRect={CGPoint=dd}{CGSize=dd}}".as_ptr(),
        );
        class_addMethod(
            cls,
            sel("mouseDown:"),
            mouse_down as *const c_void,
            c"v@:@".as_ptr(),
        );
        class_addMethod(
            cls,
            sel("isFlipped"),
            is_flipped as *const c_void,
            c"c@:".as_ptr(),
        );
        class_addMethod(
            cls,
            sel("acceptsFirstMouse:"),
            accepts_first_mouse as *const c_void,
            c"c@:@".as_ptr(),
        );
        objc_registerClassPair(cls);
        let view = msg0(msg0(cls, "alloc"), "init");

        let rect = NSRect { x: 0.0, y: 0.0, w: WIDTH, h: HEIGHT };
        let f: extern "C" fn(Id, *mut c_void, NSRect, usize, usize, bool) -> Id =
            std::mem::transmute(objc_msgSend as *const ());
        let panel = f(
            msg0(objc_getClass(c"NSPanel".as_ptr()), "alloc"),
            sel("initWithContentRect:styleMask:backing:defer:"),
            rect,
            128, // NSWindowStyleMaskNonactivatingPanel, borderless
            2,   // NSBackingStoreBuffered
            false,
        );
        msg1(panel, "setContentView:", view);
        msg_i64(panel, "setLevel:", 25);
        msg_usize(panel, "setCollectionBehavior:", 1 | 16 | 256);
        msg_bool(panel, "setOpaque:", false);
        msg_bool(panel, "setHidesOnDeactivate:", false);
        msg_bool(panel, "setCanHide:", false);
        msg_bool(panel, "setReleasedWhenClosed:", false);
        msg_bool(panel, "setMovableByWindowBackground:", true);
        msg1(panel, "setBackgroundColor:", msg0(objc_getClass(c"NSColor".as_ptr()), "clearColor"));
        let out: extern "C" fn(Id, *mut c_void, Id) =
            std::mem::transmute(objc_msgSend as *const ());
        out(panel, sel("orderOut:"), std::ptr::null_mut());
        PANEL.store(panel, Ordering::SeqCst);
        VIEW.store(view, Ordering::SeqCst);
    }
}

pub fn place(x: f64, y: f64) {
    let panel = PANEL.load(Ordering::SeqCst);
    if panel.is_null() {
        return;
    }
    unsafe {
        let f: extern "C" fn(Id, *mut c_void, NSRect, bool) =
            std::mem::transmute(objc_msgSend as *const ());
        f(
            panel,
            sel("setFrame:display:"),
            NSRect { x, y, w: WIDTH, h: HEIGHT },
            true,
        );
    }
}

pub fn show(model: Model) {
    if let Ok(mut m) = MODEL.lock() {
        *m = model;
    }
    let panel = PANEL.load(Ordering::SeqCst);
    let view = VIEW.load(Ordering::SeqCst);
    if panel.is_null() || view.is_null() {
        return;
    }
    unsafe {
        msg0(panel, "orderFrontRegardless");
        let f: extern "C" fn(Id, *mut c_void, bool) =
            std::mem::transmute(objc_msgSend as *const ());
        f(view, sel("setNeedsDisplay:"), true);
    }
}

pub fn hide() {
    let panel = PANEL.load(Ordering::SeqCst);
    if panel.is_null() {
        return;
    }
    unsafe {
        let f: extern "C" fn(Id, *mut c_void, Id) =
            std::mem::transmute(objc_msgSend as *const ());
        f(panel, sel("orderOut:"), std::ptr::null_mut());
    }
    if let Ok(mut m) = MODEL.lock() {
        m.kind = 0;
    }
}

/// `-[NSWindow frame]`. An `NSRect` is 32 bytes, which Intel returns through a
/// hidden pointer and so needs `objc_msgSend_stret`; Apple Silicon returns it
/// in registers through plain `objc_msgSend`.
#[cfg(target_arch = "x86_64")]
unsafe fn msg_frame(obj: Id) -> NSRect {
    extern "C" {
        fn objc_msgSend_stret();
    }
    let f: extern "C" fn(*mut NSRect, Id, *mut c_void) =
        std::mem::transmute(objc_msgSend_stret as *const ());
    let mut r = NSRect::default();
    f(&mut r, obj, sel("frame"));
    r
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn msg_frame(obj: Id) -> NSRect {
    let f: extern "C" fn(Id, *mut c_void) -> NSRect = std::mem::transmute(objc_msgSend as *const ());
    f(obj, sel("frame"))
}

pub fn origin() -> Option<(f64, f64)> {
    let panel = PANEL.load(Ordering::SeqCst);
    if panel.is_null() {
        return None;
    }
    unsafe {
        let r = msg_frame(panel);
        Some((r.x, r.y))
    }
}

pub fn take_actions() -> Vec<Action> {
    ACTIONS
        .lock()
        .map(|mut q| std::mem::take(&mut *q))
        .unwrap_or_default()
}

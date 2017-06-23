use cocoa::appkit::{self, NSApplication, NSColor, NSEvent, NSView, NSWindow};
use cocoa::base::{id, nil, YES};
use cocoa::foundation::{NSAutoreleasePool, NSDate, NSDefaultRunLoopMode, NSPoint, NSRect, NSSize,
                        NSString, NSUInteger};
use events;
use Event;
use Window;
use events::ElementState;
use events::{MouseButton, TouchPhase};
use std::collections::VecDeque;
use std::str::from_utf8;
use std::sync::Mutex;
use std::ffi::CStr;

static mut SHIFT_PRESSED: bool = false;
static mut CTRL_PRESSED: bool = false;
static mut WIN_PRESSED: bool = false;
static mut ALT_PRESSED: bool = false;

#[allow(non_snake_case, non_upper_case_globals)]
pub unsafe fn NSEventToEvent(nsevent: id) -> Option<Event> {
    if nsevent == nil { return None; }

    let event_type = nsevent.eventType();
    appkit::NSApp().sendEvent_(if let appkit::NSKeyDown = event_type { nil } else { nsevent });

    match event_type {
        appkit::NSLeftMouseDown         => { Some(Event::MouseInput(ElementState::Pressed, MouseButton::Left)) },
        appkit::NSLeftMouseUp           => { Some(Event::MouseInput(ElementState::Released, MouseButton::Left)) },
        appkit::NSRightMouseDown        => { Some(Event::MouseInput(ElementState::Pressed, MouseButton::Right)) },
        appkit::NSRightMouseUp          => { Some(Event::MouseInput(ElementState::Released, MouseButton::Right)) },
        appkit::NSOtherMouseDown        => { Some(Event::MouseInput(ElementState::Pressed, MouseButton::Middle)) },
        appkit::NSOtherMouseUp          => { Some(Event::MouseInput(ElementState::Released, MouseButton::Middle)) },
        appkit::NSMouseEntered          => { Some(Event::MouseEntered) },
        appkit::NSMouseExited           => { Some(Event::MouseLeft) },
        // appkit::NSMouseMoved            |
        // appkit::NSLeftMouseDragged      |
        // appkit::NSOtherMouseDragged,    |
        // appkit::NSRightMouseDragged     => {
        //     let window_point = nsevent.locationInWindow();
        //     let cWindow: id = msg_send![nsevent, window];
        //     // let view_point = if cWindow == nil {
        //     //     let window_rect = window.window.convertRectFromScreen_(NSRect::new(window_point, NSSize::new(0.0, 0.0)));
        //     //     window.view.convertPoint_fromView_(window_rect.origin, nil)
        //     // } else {
        //     //     window.view.convertPoint_fromView_(window_point, nil)
        //     // };
        //     // let view_rect = NSView::frame(*window.view);
        //     // let scale_factor = window.hidpi_factor();

        //     Some(Event::MouseMoved((scale_factor * view_point.x as f32) as i32,
        //                            (scale_factor * (view_rect.size.height - view_point.y) as f32) as i32))
        // },
        appkit::NSKeyDown => {
            let mut events = VecDeque::new();
            let received_c_str = nsevent.characters().UTF8String();
            let received_str = CStr::from_ptr(received_c_str);
            for received_char in from_utf8(received_str.to_bytes()).unwrap().chars() {
                events.push_back(Event::ReceivedCharacter(received_char));
            }

            let vkey =  vkeycode_to_element(NSEvent::keyCode(nsevent));
            events.push_back(Event::KeyboardInput(ElementState::Pressed, NSEvent::keyCode(nsevent) as u8, vkey));
            let event = events.pop_front();
            // window.delegate.state.pending_events.lock().unwrap().extend(events.into_iter());
            event
        },
        appkit::NSKeyUp => {
            let vkey =  vkeycode_to_element(NSEvent::keyCode(nsevent));

            Some(Event::KeyboardInput(ElementState::Released, NSEvent::keyCode(nsevent) as u8, vkey))
        },
        appkit::NSFlagsChanged => {
            let mut events = VecDeque::new();
            let shift_modifier = Window::modifier_event(nsevent, appkit::NSShiftKeyMask, events::VirtualKeyCode::LShift, SHIFT_PRESSED);
            if shift_modifier.is_some() {
                SHIFT_PRESSED = !SHIFT_PRESSED;
                events.push_back(shift_modifier.unwrap());
            }
            let ctrl_modifier = Window::modifier_event(nsevent, appkit::NSControlKeyMask, events::VirtualKeyCode::LControl, CTRL_PRESSED);
            if ctrl_modifier.is_some() {
                CTRL_PRESSED = !CTRL_PRESSED;
                events.push_back(ctrl_modifier.unwrap());
            }
            let win_modifier = Window::modifier_event(nsevent, appkit::NSCommandKeyMask, events::VirtualKeyCode::LWin, WIN_PRESSED);
            if win_modifier.is_some() {
                WIN_PRESSED = !WIN_PRESSED;
                events.push_back(win_modifier.unwrap());
            }
            let alt_modifier = Window::modifier_event(nsevent, appkit::NSAlternateKeyMask, events::VirtualKeyCode::LAlt, ALT_PRESSED);
            if alt_modifier.is_some() {
                ALT_PRESSED = !ALT_PRESSED;
                events.push_back(alt_modifier.unwrap());
            }
            let event = events.pop_front();
            // window.delegate.state.pending_events.lock().unwrap().extend(events.into_iter());
            event
        },
        // appkit::NSScrollWheel => {
        //     use events::MouseScrollDelta::{LineDelta, PixelDelta};
        //     let scale_factor = window.hidpi_factor();
        //     let delta = if nsevent.hasPreciseScrollingDeltas() == YES {
        //         PixelDelta(scale_factor * nsevent.scrollingDeltaX() as f32,
        //                    scale_factor * nsevent.scrollingDeltaY() as f32)
        //     } else {
        //         LineDelta(scale_factor * nsevent.scrollingDeltaX() as f32,
        //                   scale_factor * nsevent.scrollingDeltaY() as f32)
        //     };
        //     let phase = match nsevent.phase() {
        //         appkit::NSEventPhaseMayBegin | appkit::NSEventPhaseBegan => TouchPhase::Started,
        //         appkit::NSEventPhaseEnded => TouchPhase::Ended,
        //         _ => TouchPhase::Moved,
        //     };
        //     Some(Event::MouseWheel(delta, phase))
        // },
        appkit::NSEventTypePressure => {
            Some(Event::TouchpadPressure(nsevent.pressure(), nsevent.stage()))
        },
        appkit::NSApplicationDefined => {
            match nsevent.subtype() {
                appkit::NSEventSubtype::NSApplicationActivatedEventType => { Some(Event::Awakened) }
                _ => { None }
            }
        },
        _  => { None },
    }
}

pub fn vkeycode_to_element(code: u16) -> Option<events::VirtualKeyCode> {
    Some(match code {
        0x00 => events::VirtualKeyCode::A,
        0x01 => events::VirtualKeyCode::S,
        0x02 => events::VirtualKeyCode::D,
        0x03 => events::VirtualKeyCode::F,
        0x04 => events::VirtualKeyCode::H,
        0x05 => events::VirtualKeyCode::G,
        0x06 => events::VirtualKeyCode::Z,
        0x07 => events::VirtualKeyCode::X,
        0x08 => events::VirtualKeyCode::C,
        0x09 => events::VirtualKeyCode::V,
        //0x0a => World 1,
        0x0b => events::VirtualKeyCode::B,
        0x0c => events::VirtualKeyCode::Q,
        0x0d => events::VirtualKeyCode::W,
        0x0e => events::VirtualKeyCode::E,
        0x0f => events::VirtualKeyCode::R,
        0x10 => events::VirtualKeyCode::Y,
        0x11 => events::VirtualKeyCode::T,
        0x12 => events::VirtualKeyCode::Key1,
        0x13 => events::VirtualKeyCode::Key2,
        0x14 => events::VirtualKeyCode::Key3,
        0x15 => events::VirtualKeyCode::Key4,
        0x16 => events::VirtualKeyCode::Key6,
        0x17 => events::VirtualKeyCode::Key5,
        0x18 => events::VirtualKeyCode::Equals,
        0x19 => events::VirtualKeyCode::Key9,
        0x1a => events::VirtualKeyCode::Key7,
        0x1b => events::VirtualKeyCode::Minus,
        0x1c => events::VirtualKeyCode::Key8,
        0x1d => events::VirtualKeyCode::Key0,
        0x1e => events::VirtualKeyCode::RBracket,
        0x1f => events::VirtualKeyCode::O,
        0x20 => events::VirtualKeyCode::U,
        0x21 => events::VirtualKeyCode::LBracket,
        0x22 => events::VirtualKeyCode::I,
        0x23 => events::VirtualKeyCode::P,
        0x24 => events::VirtualKeyCode::Return,
        0x25 => events::VirtualKeyCode::L,
        0x26 => events::VirtualKeyCode::J,
        0x27 => events::VirtualKeyCode::Apostrophe,
        0x28 => events::VirtualKeyCode::K,
        0x29 => events::VirtualKeyCode::Semicolon,
        0x2a => events::VirtualKeyCode::Backslash,
        0x2b => events::VirtualKeyCode::Comma,
        0x2c => events::VirtualKeyCode::Slash,
        0x2d => events::VirtualKeyCode::N,
        0x2e => events::VirtualKeyCode::M,
        0x2f => events::VirtualKeyCode::Period,
        0x30 => events::VirtualKeyCode::Tab,
        0x31 => events::VirtualKeyCode::Space,
        0x32 => events::VirtualKeyCode::Grave,
        0x33 => events::VirtualKeyCode::Back,
        //0x34 => unkown,
        0x35 => events::VirtualKeyCode::Escape,
        0x36 => events::VirtualKeyCode::RWin,
        0x37 => events::VirtualKeyCode::LWin,
        0x38 => events::VirtualKeyCode::LShift,
        //0x39 => Caps lock,
        //0x3a => Left alt,
        0x3b => events::VirtualKeyCode::LControl,
        0x3c => events::VirtualKeyCode::RShift,
        //0x3d => Right alt,
        0x3e => events::VirtualKeyCode::RControl,
        //0x3f => Fn key,
        //0x40 => F17 Key,
        0x41 => events::VirtualKeyCode::Decimal,
        //0x42 -> unkown,
        0x43 => events::VirtualKeyCode::Multiply,
        //0x44 => unkown,
        0x45 => events::VirtualKeyCode::Add,
        //0x46 => unkown,
        0x47 => events::VirtualKeyCode::Numlock,
        //0x48 => KeypadClear,
        0x49 => events::VirtualKeyCode::VolumeUp,
        0x4a => events::VirtualKeyCode::VolumeDown,
        0x4b => events::VirtualKeyCode::Divide,
        0x4c => events::VirtualKeyCode::NumpadEnter,
        //0x4d => unkown,
        0x4e => events::VirtualKeyCode::Subtract,
        //0x4f => F18 key,
        //0x50 => F19 Key,
        0x51 => events::VirtualKeyCode::NumpadEquals,
        0x52 => events::VirtualKeyCode::Numpad0,
        0x53 => events::VirtualKeyCode::Numpad1,
        0x54 => events::VirtualKeyCode::Numpad2,
        0x55 => events::VirtualKeyCode::Numpad3,
        0x56 => events::VirtualKeyCode::Numpad4,
        0x57 => events::VirtualKeyCode::Numpad5,
        0x58 => events::VirtualKeyCode::Numpad6,
        0x59 => events::VirtualKeyCode::Numpad7,
        //0x5a => F20 Key,
        0x5b => events::VirtualKeyCode::Numpad8,
        0x5c => events::VirtualKeyCode::Numpad9,
        //0x5d => unkown,
        //0x5e => unkown,
        //0x5f => unkown,
        0x60 => events::VirtualKeyCode::F5,
        0x61 => events::VirtualKeyCode::F6,
        0x62 => events::VirtualKeyCode::F7,
        0x63 => events::VirtualKeyCode::F3,
        0x64 => events::VirtualKeyCode::F8,
        0x65 => events::VirtualKeyCode::F9,
        //0x66 => unkown,
        0x67 => events::VirtualKeyCode::F11,
        //0x68 => unkown,
        0x69 => events::VirtualKeyCode::F13,
        //0x6a => F16 Key,
        0x6b => events::VirtualKeyCode::F14,
        //0x6c => unkown,
        0x6d => events::VirtualKeyCode::F10,
        //0x6e => unkown,
        0x6f => events::VirtualKeyCode::F12,
        //0x70 => unkown,
        0x71 => events::VirtualKeyCode::F15,
        0x72 => events::VirtualKeyCode::Insert,
        0x73 => events::VirtualKeyCode::Home,
        0x74 => events::VirtualKeyCode::PageUp,
        0x75 => events::VirtualKeyCode::Delete,
        0x76 => events::VirtualKeyCode::F4,
        0x77 => events::VirtualKeyCode::End,
        0x78 => events::VirtualKeyCode::F2,
        0x79 => events::VirtualKeyCode::PageDown,
        0x7a => events::VirtualKeyCode::F1,
        0x7b => events::VirtualKeyCode::Left,
        0x7c => events::VirtualKeyCode::Right,
        0x7d => events::VirtualKeyCode::Down,
        0x7e => events::VirtualKeyCode::Up,
        //0x7f =>  unkown,

        _ => return None,
    })
}
use cocoa;
use cocoa::base::{id, nil};

use objc::runtime;
use objc::runtime::{BOOL, NO, YES};
use objc::runtime::{Class, Object, Sel};
use objc::declare::ClassDecl;

use std::os::raw::c_void;
use std::sync::Mutex; // Mutex<VecDeque<Event>>
use std::collections::VecDeque;

use Event;
use ElementState;
use MouseButton;

// pub fn get_window_responder_class<T>(responder: T) -> *const Class where T : EventResponder {
pub fn get_window_responder_class() -> *const Class {

    use std::sync::{Once, ONCE_INIT};

    static mut RESPONDER_CLASS: *const Class = 0 as *const Class;
    static INIT: Once = ONCE_INIT;

    // let callback = fn foo() { info!("hi"); };

    INIT.call_once(|| unsafe {
        let superclass = Class::get("NSView").unwrap();
        let mut decl = ClassDecl::new("ViewResponder", superclass).unwrap();

        decl.add_ivar::<*mut c_void>("pendingEvents");

        extern "C" fn acceptsFirstResponder(_: &Object, _: Sel) -> BOOL {
            info!("acceptsFirstResponder() hit");
            YES
        }

        // func acceptsFirstMouse(for event: NSEvent?) -> Bool
        extern "C" fn acceptsFirstMouse(_: &Object, _: Sel, theEvent: id) -> BOOL {
            info!("acceptsFirstMouse() hit");
            YES
        }

        extern "C" fn timerFired(_: &Object, _: Sel, _: id) {
            info!("timer fired - PING!");
            // event_responder.handle_event();

        }

        extern "C" fn mouseEvent(this: &Object, _: Sel, nsevent: id) {
            use cocoa::appkit;
            use cocoa::appkit::{ NSEvent, NSView, NSWindow };
            use cocoa::foundation::{ NSRect, NSSize };

            let event_type = unsafe { NSEvent::eventType(nsevent) };
            // info!("NSEvent:{:?}", event_type);

            let pe_ptr: *mut c_void = unsafe { *this.get_ivar("pendingEvents") };
            let pe = unsafe { &mut *(pe_ptr as *mut VecDeque<Event>) };

            let event = match event_type {
                appkit::NSLeftMouseDown         => { Some(Event::MouseInput(ElementState::Pressed, MouseButton::Left)) },
                appkit::NSLeftMouseUp           => { Some(Event::MouseInput(ElementState::Released, MouseButton::Left)) },
                appkit::NSRightMouseDown        => { Some(Event::MouseInput(ElementState::Pressed, MouseButton::Right)) },
                appkit::NSRightMouseUp          => { Some(Event::MouseInput(ElementState::Released, MouseButton::Right)) },
                appkit::NSOtherMouseDown        => { Some(Event::MouseInput(ElementState::Pressed, MouseButton::Middle)) },
                appkit::NSOtherMouseUp          => { Some(Event::MouseInput(ElementState::Released, MouseButton::Middle)) },
                appkit::NSMouseEntered          => { Some(Event::MouseEntered) },
                appkit::NSMouseExited           => { Some(Event::MouseLeft) },
                appkit::NSMouseMoved            |
                appkit::NSLeftMouseDragged      |
                appkit::NSOtherMouseDragged     |
                appkit::NSRightMouseDragged     => {
                    let window_point = unsafe { nsevent.locationInWindow() };
                    let cWindow: id = unsafe { msg_send![nsevent, window] };
                    let cView: id = unsafe { msg_send![cWindow, contentView] };
                    let scale_factor = hidpi_factor(cWindow);

                    Some(Event::MouseMoved((window_point.x as f32 * scale_factor) as i32,
                                            (((unsafe { NSView::frame(cView).size.height } - window_point.y) as f32 * scale_factor) as i32)))
                },

                _  => { None },
            };

            if let Some(ev) = event {
                // info!("Event stored: NSEvent:{:?} Event:{:?}", event_type, ev);
                pe.push_back(ev);
            }
        }

        decl.add_method(sel!(acceptsFirstResponder),
            acceptsFirstResponder as extern fn(this: &Object, _: Sel) -> BOOL);

        decl.add_method(sel!(acceptsFirstMouse:),
            acceptsFirstMouse as extern fn(this: &Object, _: Sel, _: id) -> BOOL);

        decl.add_method(sel!(timerFired:),
            timerFired as extern fn(this: &Object, _: Sel, _: id));

        // func mouseDown(with event: NSEvent)
        // https://developer.apple.com/documentation/appkit/nsresponder/1524634-mousedown
        decl.add_method(sel!(mouseDown:),
            mouseEvent as extern fn(this: &Object, _: Sel, _: id));

        decl.add_method(sel!(mouseUp:),
            mouseEvent as extern fn(this: &Object, _: Sel, _: id));

        decl.add_method(sel!(mouseMoved:),
            mouseEvent as extern fn(this: &Object, _: Sel, _: id));

        decl.add_method(sel!(mouseDragged:),
            mouseEvent as extern fn(this: &Object, _: Sel, _: id));

        RESPONDER_CLASS = decl.register();
    });
    unsafe { RESPONDER_CLASS }
}

fn hidpi_factor(window: id) -> f32 {
    use cocoa::appkit::NSWindow;
    unsafe {
        NSWindow::backingScaleFactor(window) as f32
    }
}

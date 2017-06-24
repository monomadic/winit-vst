use cocoa;
use cocoa::base::{id, nil};

use objc::runtime;
use objc::runtime::{BOOL, NO, YES};
use objc::runtime::{Class, Object, Sel};
use objc::declare::ClassDecl;

use std::os::raw::c_void;
use std::sync::Mutex; // Mutex<VecDeque<Event>>
use std::collections::VecDeque;

// use platform::platform::event_responder::*;
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

        // decl.add_ivar::<*mut c_void>("ViewController");
        // decl.add_ivar::<*mut c_void>("EventCallbacks");
        decl.add_ivar::<*mut c_void>("eventHandler");
        // decl.add_ivar::<*mut c_void>("theTimer");

        decl.add_ivar::<*mut c_void>("pendingEvents");

        // extern "C" fn setViewController(this: &mut Object, _: Sel, controller: *mut c_void) {
        //     unsafe {
        //         this.set_ivar("ViewController", controller);
        //     }
        // }
        // extern "C" fn setEventCallbacks(this: &mut Object, _: Sel, handler: *mut c_void) {
        //     unsafe {
        //         this.set_ivar("EventCallbacks", handler);
        //     }
        // }
        extern "C" fn setPendingEvents(this: &mut Object, _: Sel, vec: *mut c_void) {
            unsafe {
                this.set_ivar("pendingEvents", vec);
            }
        }

        // @property(readonly) BOOL acceptsFirstResponder;
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

        extern "C" fn startTimer(_: &Object, _: Sel) {
            info!("stopping timer.");
            // unsafe { msg_send![, release] };
        }

        extern "C" fn stopTimer(_: &Object, _: Sel) {
            info!("stopping timer.");
            // unsafe { msg_send![, release] };
        }

        extern "C" fn mouseEvent(this: &Object, _: Sel, mouseEvent: id) {
            use cocoa::appkit::NSEvent;
            info!("NSEvent type: {:?}", unsafe { NSEvent::eventType(mouseEvent) });
        }

        // decl.add_method(sel!(setEventCallbacks:),
        //     setEventCallbacks as extern "C" fn(this: &mut Object, _: Sel, _: *mut c_void));
        // decl.add_method(sel!(setViewController:),
        //                 setViewController as
        //                 extern "C" fn(this: &mut Object, _: Sel, _: *mut c_void));

        decl.add_method(sel!(setPendingEvents:),
                        setPendingEvents as
                        extern "C" fn(this: &mut Object, _: Sel, _: *mut c_void));

        decl.add_method(sel!(acceptsFirstResponder),
            acceptsFirstResponder as extern fn(this: &Object, _: Sel) -> BOOL);

        decl.add_method(sel!(acceptsFirstMouse:),
            acceptsFirstMouse as extern fn(this: &Object, _: Sel, _: id) -> BOOL);

        decl.add_method(sel!(timerFired:),
            timerFired as extern fn(this: &Object, _: Sel, _: id));

        // decl.add_method(sel!(stopTimer),
        //     stopTimer as extern fn(this: &Object, _: Sel));

        // func mouseDown(with event: NSEvent)
        // https://developer.apple.com/documentation/appkit/nsresponder/1524634-mousedown
        decl.add_method(sel!(mouseDown:),
            mouseEvent as extern fn(this: &Object, _: Sel, _: id));

        decl.add_method(sel!(mouseUp:),
            mouseEvent as extern fn(this: &Object, _: Sel, _: id));

        RESPONDER_CLASS = decl.register();
    });
    unsafe { RESPONDER_CLASS }
}

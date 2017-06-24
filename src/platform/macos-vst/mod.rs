#![cfg(target_os = "macos")]

use objc;
use cocoa::base::{id, nil, YES, NO, SEL, class};
use libc;

use std::os::raw::c_void;
use std::sync::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use os::macos::{ ActivationPolicy, WindowExt };

use CreationError;
use CursorState;
use Event;
use MouseCursor;
use WindowAttributes;

mod idref;
use self::idref::IdRef;
mod responder;
mod event_responder;
pub use self::event_responder::*;

#[derive(Clone, Default)]
pub struct PlatformSpecificWindowBuilderAttributes {
    pub activation_policy: ActivationPolicy,
}

#[derive(Clone)]
pub struct WindowProxy;

impl WindowProxy {
    #[inline]
    pub fn wakeup_event_loop(&self) {
        unimplemented!()
    }
}

#[derive(Clone, Copy)]
pub struct MonitorId;

#[inline]
pub fn get_available_monitors() -> VecDeque<MonitorId> {
    let mut list = VecDeque::new();
    list.push_back(MonitorId);
    list
}

#[inline]
pub fn get_primary_monitor() -> MonitorId {
    MonitorId
}

impl MonitorId {
    #[inline]
    pub fn get_name(&self) -> Option<String> {
        Some("Canvas".to_owned())
    }

    #[inline]
    pub fn get_native_identifier(&self) -> ::native_monitor::NativeMonitorId {
        ::native_monitor::NativeMonitorId::Unavailable
    }

    #[inline]
    pub fn get_dimensions(&self) -> (u32, u32) {
        unimplemented!()
    }
}


pub struct PollEventsIterator<'a> {
    window: &'a Window
}

use cocoa::foundation::{ NSAutoreleasePool, NSDate, NSDefaultRunLoopMode };
use cocoa::appkit;
use cocoa::appkit::{ NSApplication, NSColor, NSEvent, NSView, NSWindow };

impl<'a> Iterator for PollEventsIterator<'a> {
    type Item = Event;

    #[inline]
    fn next(&mut self, ) -> Option<Event> {
        let event: Option<Event>;
        unsafe {
            let pool = NSAutoreleasePool::new(nil);

            let nsevent = appkit::NSApp().nextEventMatchingMask_untilDate_inMode_dequeue_(
                appkit::NSAnyEventMask.bits() | appkit::NSEventMaskPressure.bits(),
                NSDate::distantPast(nil),
                NSDefaultRunLoopMode,
                YES);

            event = NSEventToEvent(self.window, nsevent);

            let _: () = msg_send![pool, release];
        }
        event
    }
}

pub unsafe fn NSEventToEvent(window: &Window, nsevent: id) -> Option<Event> {
    if nsevent == nil { return None; }

    let event_type = nsevent.eventType();
    None
}

pub struct WaitEventsIterator<'a> {
    window: &'a Window
}

impl<'a> Iterator for WaitEventsIterator<'a> {
    type Item = Event;

    #[inline]
    fn next(&mut self) -> Option<Event> {
        None
    }
}

pub struct Window {
    window: IdRef,
    host_view: IdRef,
    view: IdRef,
    timer: IdRef,
    pending_events: Box<VecDeque<Event>>,
}

impl Drop for Window {
    fn drop(&mut self) {
        info!("dropping window.");
        info!("invalidating timer...");
        unsafe { msg_send![*self.timer, invalidate] };
        info!("stopped timer!");
    }
}

impl WindowExt for Window {
    #[inline]
    fn get_nswindow(&self) -> *mut c_void {
        warn!("raw pointer to nswindow requested!");
        *self.window as *mut c_void
    }

    #[inline]
    fn get_nsview(&self) -> *mut c_void {
        warn!("raw pointer to nsview requested!");
        *self.view as *mut c_void
    }
}

impl Window {
    #[inline]
    pub fn new(win_attribs: &WindowAttributes,
               _: &PlatformSpecificWindowBuilderAttributes)
                -> Result<Window, CreationError> {

        use cocoa::appkit::{ NSWindow, NSView };

        // logging
        use simplelog::*;
        use std::fs::File;
        let _ = CombinedLogger::init(
            vec![
                WriteLogger::new(LogLevelFilter::Info, Config::default(), File::create("/tmp/simplesynth.log").unwrap()),
            ]
        );
        use log_panics;
        log_panics::init();
        info!("Winit logging started. Attaching new handle.");

        match win_attribs.parent {
            Some(parent) => {
                let host_view_id = parent as id;
                let window = unsafe{ msg_send![host_view_id, window] };
                // let event_responder = EventResponder{};
                // let pop = || { info!("poppp!") };


                let view: id = unsafe { msg_send![responder::get_window_responder_class(), new] };

                let mut pending_events = Box::new(VecDeque::new());
                let pending_events_ptr: *mut VecDeque<Event> = &mut *pending_events;
                unsafe {
                    // msg_send![view, setPendingEvents:(pending_events_ptr as *mut c_void)];
                    (&mut *view).set_ivar("pendingEvents", pending_events_ptr as *mut ::std::os::raw::c_void);
                }

                // pub struct MyController {}
                // impl ViewController for MyController {
                //     fn on_mouse_down(&mut self) {
                //         info!("Yaaaas");
                //     }
                // }
                // let mut controller = Controller::new(Box::new(MyController {}));
                // set_view_controller(view, &mut controller);

                // fn set_view_controller(view: id, controller: *mut Controller) {
                //     unsafe {
                //         msg_send![view, setViewController:(controller as *mut c_void)];
                //     }
                // }

                use objc::runtime::{Class};

                info!("creating timer");
                let timer: id = unsafe { msg_send![ Class::get("NSTimer").unwrap(),
                        scheduledTimerWithTimeInterval:0.2
                        target:view
                        selector:sel!(timerFired:)
                        userInfo:nil
                        repeats:YES
                    ]
                };
                info!("timer created.");
                // Timer { id: IdRef::new(timer) };
                // unsafe { msg_send![*self.timer, invalidate] };
                
                unsafe {
                    NSView::addSubview_(host_view_id, view);
                    NSWindow::setContentView_(window, view);
                    NSWindow::makeKeyAndOrderFront_(window, nil);
                    NSView::setWantsBestResolutionOpenGLSurface_(view, YES);
                };

                info!("created window: {:?}", window as id);

                Ok(Window{
                    window: IdRef::retain(window),
                    host_view: IdRef::retain(host_view_id),
                    view: IdRef::new(view),
                    // controller: controller,
                    timer: IdRef::retain(timer),
                    pending_events: pending_events,
                })
            },
            None => Err(CreationError::OsError("Parent view is null.".to_string()))
        }
    }

    #[inline]
    pub fn set_title(&self, _title: &str) {
        error!("set_title() not supported.");
    }

    #[inline]
    pub fn get_position(&self) -> Option<(i32, i32)> {
        unsafe {
            use cocoa::appkit::NSWindow;
            use cocoa::foundation::{ NSRect, NSPoint, NSSize };
            use core_graphics::display::{ CGDisplayPixelsHigh, CGMainDisplayID };

            let content_rect = NSWindow::contentRectForFrameRect_(*self.window, NSWindow::frame(*self.window));
            Some((content_rect.origin.x as i32, (CGDisplayPixelsHigh(CGMainDisplayID()) as f64 - (content_rect.origin.y + content_rect.size.height)) as i32))
        }
    }

    #[inline]
    pub fn set_position(&self, _: i32, _: i32) {
        error!("set_position() not supported.");
    }

    pub fn get_inner_size(&self) -> Option<(u32, u32)> {
        use cocoa::appkit::NSView;
        unsafe {
            let view_frame = NSView::frame(*self.view);
            Some((view_frame.size.width as u32, view_frame.size.height as u32))
        }
    }

    #[inline]
    pub fn get_outer_size(&self) -> Option<(u32, u32)> {
        use cocoa::appkit::NSWindow;
        unsafe {
            let window_frame = NSWindow::frame(*self.window);
            Some((window_frame.size.width as u32, window_frame.size.height as u32))
        }
    }

    #[inline]
    pub fn set_inner_size(&self, width: u32, height: u32) {
        use cocoa::appkit::NSWindow;
        use cocoa::foundation::NSSize;
        unsafe {
            NSWindow::setContentSize_(*self.window, NSSize::new(width as f64, height as f64));
        }
    }

    #[inline]
    pub fn poll_events(&self) -> PollEventsIterator {
        PollEventsIterator {
            window: self,
        }
    }

    #[inline]
    pub fn wait_events(&self) -> WaitEventsIterator {
        WaitEventsIterator {
            window: self,
        }
    }

    #[inline]
    pub fn create_window_proxy(&self) -> WindowProxy {
        WindowProxy
    }

    #[inline]
    pub fn show(&self) {}
    #[inline]
    pub fn hide(&self) {}

    #[inline]
    pub fn platform_display(&self) -> *mut libc::c_void {
        *self.view as *mut libc::c_void
    }

    #[inline]
    pub fn platform_window(&self) -> *mut libc::c_void {
        // warn!("platform_window() requested!");
        *self.window as *mut libc::c_void
    }

    #[inline]
    pub fn set_window_resize_callback(&mut self, _: Option<fn(u32, u32)>) {
    }

    #[inline]
    pub fn set_cursor(&self, cursor: MouseCursor) {
    }

    #[inline]
    pub fn set_cursor_state(&self, state: CursorState) -> Result<(), String> {
        Ok(())
    }

    #[inline]
    pub fn hidpi_factor(&self) -> f32 {
        use cocoa::appkit::NSWindow;
        unsafe {
            NSWindow::backingScaleFactor(*self.window) as f32
        }
    }

    #[inline]
    pub fn set_cursor_position(&self, x: i32, y: i32) -> Result<(), ()> {
        Ok(())
    }
}

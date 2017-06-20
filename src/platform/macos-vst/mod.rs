#![cfg(target_os = "macos")]

mod idref;
use self::idref::IdRef;

use cocoa::base::{id, nil};

use std::collections::VecDeque;
use std::sync::Arc;

use CreationError;
use CursorState;
use Event;
use MouseCursor;
use WindowAttributes;
use libc;

use std::os::raw::c_void;

use os::macos::{ ActivationPolicy, WindowExt };

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

impl<'a> Iterator for PollEventsIterator<'a> {
    type Item = Event;

    #[inline]
    fn next(&mut self) -> Option<Event> {
        None
    }
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
    view: IdRef,
    // host_view: id,
}

impl WindowExt for Window {
    #[inline]
    fn get_nswindow(&self) -> *mut c_void {
        // allocs a nil to send back. might break stuff later if they need the window.
        // *IdRef::new(nil) as *mut c_void
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

        match win_attribs.parent {
            Some(parent) => {
                let host_view_id = parent as id;
                // let window = unsafe{ msg_send![host_view_id, window] };

                Ok(Window{
                    // window: IdRef::retain(window),
                    window: IdRef::new(nil),
                    view: IdRef::retain(host_view_id),
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
        Some((0, 0))
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
        self.get_inner_size()
    }

    #[inline]
    pub fn set_inner_size(&self, width: u32, height: u32) {
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
        unimplemented!()
    }

    #[inline]
    pub fn platform_window(&self) -> *mut libc::c_void {
        unimplemented!()
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
        1.0
    }

    #[inline]
    pub fn set_cursor_position(&self, x: i32, y: i32) -> Result<(), ()> {
        Ok(())
    }
}

use std::sync::{Arc, Mutex};
use std::io;
use std::ptr;
use std::mem;
use std::thread;

use super::callback;
use super::WindowState;
use super::Window;
use super::MonitorId;
use super::WindowWrapper;
use super::PlatformSpecificWindowBuilderAttributes;

use CreationError;
use CreationError::OsError;
use CursorState;
use WindowAttributes;
use platform;

use std::ffi::{OsStr};
use std::os::windows::ffi::OsStrExt;
use std::sync::mpsc::channel;
use std::os::raw::c_void;

use winapi;
use kernel32;
use dwmapi;
use user32;

pub fn new_window(window: &WindowAttributes, pl_attribs: &PlatformSpecificWindowBuilderAttributes) -> Result<Window, CreationError> {
    let window = window.clone();
    let attribs = pl_attribs.clone();
    // initializing variables to be sent to the task

    let title = OsStr::new(&window.title).encode_wide().chain(Some(0).into_iter())
                                          .collect::<Vec<_>>();

    let (tx, rx) = channel();

    let window2 = WindowAttributes2::from(window.clone());

    let parent = window.parent.unwrap_or(ptr::null_mut());
    if !parent.is_null() {
        unsafe {
            // creating and sending the `Window`
            match init(title, &window2, pl_attribs.clone(), parent as winapi::HWND) {
                Ok(w) => tx.send(Ok(w)).ok(),
                Err(e) => tx.send(Err(e)).ok(),
            };
        }
    } else {
        // `GetMessage` must be called in the same thread as CreateWindow, so we create a new thread
        // dedicated to this window.
        thread::spawn(move || {
            unsafe {
                // creating and sending the `Window`
                match init(title, &window2, attribs, ptr::null_mut()) {
                    Ok(w) => tx.send(Ok(w)).ok(),
                    Err(e) => {
                        tx.send(Err(e)).ok();
                        return;
                    }
                };

                // now that the `Window` struct is initialized, the main `Window::new()` function will
                //  return and this events loop will run in parallel
                loop {
                    let mut msg = mem::uninitialized();

                    if user32::GetMessageW(&mut msg, ptr::null_mut(), 0, 0) == 0 {
                        break;
                    }

                    user32::TranslateMessage(&msg);
                    user32::DispatchMessageW(&msg);   // calls `callback` (see the callback module)
                }
            }
        });
    }

    rx.recv().unwrap()
}

unsafe fn init(title: Vec<u16>, window: &WindowAttributes2, _pl_attribs: PlatformSpecificWindowBuilderAttributes, maybe_parent: winapi::HWND) -> Result<Window, CreationError> {
    // registering the window class
    let class_name = register_window_class();

    // building a RECT object with coordinates
    let mut rect = winapi::RECT {
        left: 0, right: window.dimensions.unwrap_or((1024, 768)).0 as winapi::LONG,
        top: 0, bottom: window.dimensions.unwrap_or((1024, 768)).1 as winapi::LONG,
    };

    // switching to fullscreen if necessary
    // this means adjusting the window's position so that it overlaps the right monitor,
    //  and change the monitor's resolution if necessary
    if window.monitor.is_some() {
        let monitor = window.monitor.as_ref().unwrap();
        try!(switch_to_fullscreen(&mut rect, monitor));
    }

    // computing the style and extended style of the window
    let (ex_style, style) = if window.monitor.is_some() || !window.decorations {
        (winapi::WS_EX_APPWINDOW,
            //winapi::WS_POPUP is incompatible with winapi::WS_CHILD
            if !maybe_parent.is_null() {
                winapi::WS_CLIPSIBLINGS | winapi::WS_CLIPCHILDREN
            } else {
                winapi::WS_POPUP | winapi::WS_CLIPSIBLINGS | winapi::WS_CLIPCHILDREN
            }
        )
    } else {
        (winapi::WS_EX_APPWINDOW | winapi::WS_EX_WINDOWEDGE,
            winapi::WS_OVERLAPPEDWINDOW | winapi::WS_CLIPSIBLINGS | winapi::WS_CLIPCHILDREN)
    };

    // adjusting the window coordinates using the style
    user32::AdjustWindowRectEx(&mut rect, style, 0, ex_style);

    // creating the real window this time, by using the functions in `extra_functions`
    let real_window = {
        let (width, height) = if window.monitor.is_some() || window.dimensions.is_some() {
            (Some(rect.right - rect.left), Some(rect.bottom - rect.top))
        } else {
            (None, None)
        };

        let (x, y) = if window.monitor.is_some() {
            (Some(rect.left), Some(rect.top))
        } else {
            (None, None)
        };

        let mut style = if !window.visible {
            style
        } else {
            style | winapi::WS_VISIBLE
        };

        if !maybe_parent.is_null() {
            style |= winapi::WS_CHILD;
        }

        let handle = user32::CreateWindowExW(ex_style | winapi::WS_EX_ACCEPTFILES,
            class_name.as_ptr(),
            title.as_ptr() as winapi::LPCWSTR,
            style | winapi::WS_CLIPSIBLINGS | winapi::WS_CLIPCHILDREN,
            x.unwrap_or(winapi::CW_USEDEFAULT), y.unwrap_or(winapi::CW_USEDEFAULT),
            width.unwrap_or(winapi::CW_USEDEFAULT), height.unwrap_or(winapi::CW_USEDEFAULT),
            maybe_parent,
            ptr::null_mut(), kernel32::GetModuleHandleW(ptr::null()),
            ptr::null_mut());

        if handle.is_null() {
            return Err(OsError(format!("CreateWindowEx function failed: {}",
                                       format!("{}", io::Error::last_os_error()))));
        }

        let hdc = user32::GetDC(handle);
        if hdc.is_null() {
            return Err(OsError(format!("GetDC function failed: {}",
                                       format!("{}", io::Error::last_os_error()))));
        }

        WindowWrapper(handle, hdc)
    };

    // making the window transparent
    if window.transparent {
        let bb = winapi::DWM_BLURBEHIND {
            dwFlags: 0x1, // FIXME: DWM_BB_ENABLE;
            fEnable: 1,
            hRgnBlur: ptr::null_mut(),
            fTransitionOnMaximized: 0,
        };

        dwmapi::DwmEnableBlurBehindWindow(real_window.0, &bb);
    }

    // calling SetForegroundWindow if fullscreen
    if window.monitor.is_some() {
        user32::SetForegroundWindow(real_window.0);
    }

    let window = WindowAttributes {
        dimensions: window.dimensions,
        min_dimensions: window.min_dimensions,
        max_dimensions: window.max_dimensions,
        monitor: window.monitor.clone(),
        title: window.title.clone(),
        visible: window.visible,
        transparent: window.transparent,
        decorations: window.decorations,
        multitouch: window.multitouch,
        resize_callback: window.resize_callback,
        parent: Some(maybe_parent as *mut c_void),
    };

    // Creating a mutex to track the current window state
    let window_state = Arc::new(Mutex::new(WindowState {
        cursor: winapi::IDC_ARROW, // use arrow by default
        cursor_state: CursorState::Normal,
        attributes: window.clone()
    }));

    // filling the CONTEXT_STASH task-local storage so that we can start receiving events
    let events_receiver = {
        let (tx, rx) = channel();
        let mut tx = Some(tx);
        callback::CONTEXT_STASH.with(|context_stash| {
            let data = callback::ThreadLocalData {
                win: real_window.0,
                sender: tx.take().unwrap(),
                window_state: window_state.clone(),
                mouse_in_window: false
            };
            (*context_stash.borrow_mut()).insert(real_window.0, data);
        });
        rx
    };

    // building the struct
    Ok(Window {
        window: real_window,
        events_receiver: events_receiver,
        window_state: window_state,
    })
}

unsafe fn register_window_class() -> Vec<u16> {
    let class_name = OsStr::new("Window Class").encode_wide().chain(Some(0).into_iter())
                                               .collect::<Vec<_>>();

    let class = winapi::WNDCLASSEXW {
        cbSize: mem::size_of::<winapi::WNDCLASSEXW>() as winapi::UINT,
        style: winapi::CS_HREDRAW | winapi::CS_VREDRAW | winapi::CS_OWNDC,
        lpfnWndProc: Some(callback::callback),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: kernel32::GetModuleHandleW(ptr::null()),
        hIcon: ptr::null_mut(),
        hCursor: ptr::null_mut(),       // must be null in order for cursor state to work properly
        hbrBackground: ptr::null_mut(),
        lpszMenuName: ptr::null(),
        lpszClassName: class_name.as_ptr(),
        hIconSm: ptr::null_mut(),
    };

    // We ignore errors because registering the same window class twice would trigger
    //  an error, and because errors here are detected during CreateWindowEx anyway.
    // Also since there is no weird element in the struct, there is no reason for this
    //  call to fail.
    user32::RegisterClassExW(&class);

    class_name
}

unsafe fn switch_to_fullscreen(rect: &mut winapi::RECT, monitor: &MonitorId)
                               -> Result<(), CreationError>
{
    // adjusting the rect
    {
        let pos = monitor.get_position();
        rect.left += pos.0 as winapi::LONG;
        rect.right += pos.0 as winapi::LONG;
        rect.top += pos.1 as winapi::LONG;
        rect.bottom += pos.1 as winapi::LONG;
    }

    // changing device settings
    let mut screen_settings: winapi::DEVMODEW = mem::zeroed();
    screen_settings.dmSize = mem::size_of::<winapi::DEVMODEW>() as winapi::WORD;
    screen_settings.dmPelsWidth = (rect.right - rect.left) as winapi::DWORD;
    screen_settings.dmPelsHeight = (rect.bottom - rect.top) as winapi::DWORD;
    screen_settings.dmBitsPerPel = 32;      // TODO: ?
    screen_settings.dmFields = winapi::DM_BITSPERPEL | winapi::DM_PELSWIDTH | winapi::DM_PELSHEIGHT;

    let result = user32::ChangeDisplaySettingsExW(monitor.get_adapter_name().as_ptr(),
                                                  &mut screen_settings, ptr::null_mut(),
                                                  winapi::CDS_FULLSCREEN, ptr::null_mut());

    if result != winapi::DISP_CHANGE_SUCCESSFUL {
        return Err(OsError(format!("ChangeDisplaySettings failed: {}", result)));
    }

    Ok(())
}

#[derive(Clone)]
struct WindowAttributes2 {
    /// The dimensions of the window. If this is `None`, some platform-specific dimensions will be
    /// used.
    ///
    /// The default is `None`.
    pub dimensions: Option<(u32, u32)>,

    /// The minimum dimensions a window can be, If this is `None`, the window will have no minimum dimensions (aside from reserved).
    ///
    /// The default is `None`.
    pub min_dimensions: Option<(u32, u32)>,

    /// The maximum dimensions a window can be, If this is `None`, the maximum will have no maximum or will be set to the primary monitor's dimensions by the platform.
    ///
    /// The default is `None`.
    pub max_dimensions: Option<(u32, u32)>,

    /// If `Some`, the window will be in fullscreen mode with the given monitor.
    ///
    /// The default is `None`.
    pub monitor: Option<platform::MonitorId>,

    /// The title of the window in the title bar.
    ///
    /// The default is `"glutin window"`.
    pub title: String,

    /// Whether the window should be immediately visible upon creation.
    ///
    /// The default is `true`.
    pub visible: bool,

    /// Whether the the window should be transparent. If this is true, writing colors
    /// with alpha values different than `1.0` will produce a transparent window.
    ///
    /// The default is `false`.
    pub transparent: bool,

    /// Whether the window should have borders and bars.
    ///
    /// The default is `true`.
    pub decorations: bool,

    /// [iOS only] Enable multitouch, see [UIView#multipleTouchEnabled]
    /// (https://developer.apple.com/library/ios/documentation/UIKit/Reference/UIView_Class/#//apple_ref/occ/instp/UIView/multipleTouchEnabled)
    pub multitouch: bool,

    /// A function called upon resizing, necessary to receive resize events on Mac and possibly
    /// other systems.
    pub resize_callback: Option<fn(u32, u32)>,
}

impl From<WindowAttributes> for WindowAttributes2{
    fn from(window: WindowAttributes) -> WindowAttributes2{
        WindowAttributes2 {
            dimensions: window.dimensions,
            min_dimensions: window.min_dimensions,
            max_dimensions: window.max_dimensions,
            monitor: window.monitor,
            title: window.title,
            visible: window.visible,
            transparent: window.transparent,
            decorations: window.decorations,
            multitouch: window.multitouch,
            resize_callback: window.resize_callback,
        }
    }
}
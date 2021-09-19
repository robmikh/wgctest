use std::sync::Once;

use bindings::Windows::System::{DispatcherQueue, DispatcherQueueHandler};
use bindings::Windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, PWSTR, RECT, WPARAM};
use bindings::Windows::Win32::System::LibraryLoader::GetModuleHandleW;
use bindings::Windows::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, DestroyWindow, LoadCursorW,
    RegisterClassW, ShowWindow, CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, HMENU, IDC_ARROW,
    SW_SHOW, WINDOW_LONG_PTR_INDEX, WM_NCCREATE, WNDCLASSW, WS_EX_NOREDIRECTIONBITMAP,
    WS_OVERLAPPEDWINDOW,
};
use windows::Handle;

use crate::wide_string::ToWide;

static TEST_WINDOW_CLASS_REGISTRATION: Once = Once::new();
static TEST_WINDOW_CLASS_NAME: &str = "wgctest.TestWindow";

pub struct TestWindow {
    handle: HWND,
    queue: DispatcherQueue,
}

impl TestWindow {
    pub fn new(title: &str, width: u32, height: u32) -> windows::Result<Self> {
        let class_name = TEST_WINDOW_CLASS_NAME.to_wide();
        let instance = unsafe { GetModuleHandleW(PWSTR(std::ptr::null_mut())).ok()? };
        TEST_WINDOW_CLASS_REGISTRATION.call_once(|| {
            let class = WNDCLASSW {
                hCursor: unsafe { LoadCursorW(HINSTANCE(0), IDC_ARROW).ok().unwrap() },
                hInstance: instance,
                lpszClassName: class_name.as_pwstr(),
                lpfnWndProc: Some(Self::wnd_proc),
                ..Default::default()
            };
            assert_ne!(unsafe { RegisterClassW(&class) }, 0);
        });

        let width = width as i32;
        let height = height as i32;
        let window_ex_style = WS_EX_NOREDIRECTIONBITMAP;
        let window_style = WS_OVERLAPPEDWINDOW;

        let (adjusted_width, adjusted_height) = {
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: width as i32,
                bottom: height as i32,
            };
            unsafe {
                AdjustWindowRectEx(&mut rect, window_style, false, window_ex_style).ok()?;
            }
            (rect.right - rect.left, rect.bottom - rect.top)
        };

        let mut result = Self {
            handle: HWND(0),
            queue: DispatcherQueue::GetForCurrentThread()?,
        };

        let title = title.to_wide();
        let window = unsafe {
            CreateWindowExW(
                window_ex_style,
                class_name.as_pwstr(),
                title.as_pwstr(),
                window_style,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                adjusted_width,
                adjusted_height,
                HWND(0),
                HMENU(0),
                instance,
                &mut result as *mut _ as _,
            )
            .ok()?
        };
        result.handle = window;
        unsafe { ShowWindow(&window, SW_SHOW) };

        Ok(result)
    }

    pub fn handle(&self) -> HWND {
        self.handle
    }

    pub fn close(&self) -> windows::Result<()> {
        let handle = self.handle;
        self.queue.TryEnqueue(DispatcherQueueHandler::new(
            move || -> windows::Result<()> {
                unsafe { DestroyWindow(handle) };
                Ok(())
            },
        ))?;
        Ok(())
    }

    fn message_handler(&mut self, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        unsafe { DefWindowProcW(self.handle, message, wparam, lparam) }
    }

    unsafe extern "system" fn wnd_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if message == WM_NCCREATE {
            let cs = lparam.0 as *const CREATESTRUCTW;
            let this = (*cs).lpCreateParams as *mut Self;
            (*this).handle = window;

            SetWindowLong(window, GWLP_USERDATA, this as _);
        } else {
            let this = GetWindowLong(window, GWLP_USERDATA) as *mut Self;

            if !this.is_null() {
                return (*this).message_handler(message, wparam, lparam);
            }
        }
        DefWindowProcW(window, message, wparam, lparam)
    }
}

impl Drop for TestWindow {
    fn drop(&mut self) {
        self.close().unwrap();
    }
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "32")]
unsafe fn SetWindowLong(window: HWND, index: WINDOW_LONG_PTR_INDEX, value: isize) -> isize {
    use bindings::Windows::Win32::UI::WindowsAndMessaging::SetWindowLongW;

    SetWindowLongW(window, index, value as _) as _
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "64")]
unsafe fn SetWindowLong(window: HWND, index: WINDOW_LONG_PTR_INDEX, value: isize) -> isize {
    use bindings::Windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW;

    SetWindowLongPtrW(window, index, value)
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "32")]
unsafe fn GetWindowLong(window: HWND, index: WINDOW_LONG_PTR_INDEX) -> isize {
    use bindings::Windows::Win32::UI::WindowsAndMessaging::SetWindowLongW;

    GetWindowLongW(window, index) as _
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "64")]
unsafe fn GetWindowLong(window: HWND, index: WINDOW_LONG_PTR_INDEX) -> isize {
    use bindings::Windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW;

    GetWindowLongPtrW(window, index)
}

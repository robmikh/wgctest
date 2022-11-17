use std::sync::mpsc::channel;
use std::sync::Once;

use windows::core::HSTRING;
use windows::h;
use windows::System::{DispatcherQueue, DispatcherQueueHandler};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW,
    LoadCursorW, RegisterClassW, SetWindowLongPtrW, ShowWindow, CREATESTRUCTW, CW_USEDEFAULT,
    GWLP_USERDATA, HMENU, IDC_ARROW, SW_SHOW, WM_NCCREATE, WNDCLASSW, WS_EX_NOREDIRECTIONBITMAP,
    WS_OVERLAPPEDWINDOW,
};

use super::handle::CheckHandle;

static TEST_WINDOW_CLASS_REGISTRATION: Once = Once::new();
static TEST_WINDOW_CLASS_NAME: &HSTRING = h!("wgctest.TestWindow");

pub struct TestWindow {
    handle: HWND,
    queue: DispatcherQueue,
}

impl TestWindow {
    pub fn new_on_thread(
        dispatcher_queue: &DispatcherQueue,
        title: &'static str,
        width: u32,
        height: u32,
    ) -> windows::core::Result<Self> {
        let (sender, receiver) = channel();
        dispatcher_queue.TryEnqueue(&DispatcherQueueHandler::new(
            move || -> windows::core::Result<()> {
                let window = TestWindow::new(title, width, height)?;
                sender.send(window).unwrap();
                Ok(())
            },
        ))?;
        let window = receiver.recv().unwrap();
        Ok(window)
    }

    pub fn new(title: &str, width: u32, height: u32) -> windows::core::Result<Self> {
        let instance = unsafe { GetModuleHandleW(None)? };
        TEST_WINDOW_CLASS_REGISTRATION.call_once(|| {
            let class = WNDCLASSW {
                hCursor: unsafe { LoadCursorW(HINSTANCE(0), IDC_ARROW).ok().unwrap() },
                hInstance: instance,
                lpszClassName: TEST_WINDOW_CLASS_NAME.into(),
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

        let window = unsafe {
            CreateWindowExW(
                window_ex_style,
                TEST_WINDOW_CLASS_NAME,
                &HSTRING::from(title),
                window_style,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                adjusted_width,
                adjusted_height,
                HWND(0),
                HMENU(0),
                instance,
                Some(&mut result as *mut _ as _),
            )
            .ok()?
        };
        unsafe { ShowWindow(window, SW_SHOW) };

        Ok(result)
    }

    pub fn handle(&self) -> HWND {
        self.handle
    }

    pub fn close(&self) -> windows::core::Result<()> {
        let handle = self.handle;
        let handler = DispatcherQueueHandler::new(move || -> windows::core::Result<()> {
            unsafe { DestroyWindow(handle) };
            Ok(())
        });
        self.queue.TryEnqueue(&handler)?;
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

            SetWindowLongPtrW(window, GWLP_USERDATA, this as _);
        } else {
            let this = GetWindowLongPtrW(window, GWLP_USERDATA) as *mut Self;

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

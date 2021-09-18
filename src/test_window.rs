use std::sync::Once;

use bindings::Windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, PWSTR, RECT, WPARAM};
use bindings::Windows::Win32::System::LibraryLoader::GetModuleHandleW;
use bindings::Windows::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, DestroyWindow, LoadCursorW,
    RegisterClassW, ShowWindow, CW_USEDEFAULT, HMENU, IDC_ARROW, SW_SHOW, WNDCLASSW,
    WS_EX_NOREDIRECTIONBITMAP, WS_OVERLAPPEDWINDOW,
};
use windows::Handle;

use crate::wide_string::ToWide;

static TEST_WINDOW_CLASS_REGISTRATION: Once = Once::new();
static TEST_WINDOW_CLASS_NAME: &str = "wgctest.TestWindow";

pub struct TestWindow(pub HWND);

impl TestWindow {
    pub fn close(&self) {
        unsafe { DestroyWindow(self.0) };
    }
}

impl Drop for TestWindow {
    fn drop(&mut self) {
        self.close();
    }
}

pub fn create_test_window(title: &str, width: u32, height: u32) -> windows::Result<TestWindow> {
    let class_name = TEST_WINDOW_CLASS_NAME.to_wide();
    let instance = unsafe { GetModuleHandleW(PWSTR(std::ptr::null_mut())).ok()? };
    TEST_WINDOW_CLASS_REGISTRATION.call_once(|| {
        let class = WNDCLASSW {
            hCursor: unsafe { LoadCursorW(HINSTANCE(0), IDC_ARROW).ok().unwrap() },
            hInstance: instance,
            lpszClassName: class_name.as_pwstr(),
            lpfnWndProc: Some(def_window_proc),
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
            std::ptr::null_mut(),
        )
        .ok()?
    };

    unsafe { ShowWindow(&window, SW_SHOW) };

    Ok(TestWindow(window))
}

unsafe extern "system" fn def_window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(window, message, wparam, lparam)
}

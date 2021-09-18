use bindings::Windows::{
    Graphics::Capture::GraphicsCaptureItem,
    Win32::{
        Foundation::HWND,
        System::WinRT::{ICompositorDesktopInterop, IGraphicsCaptureItemInterop},
    },
    UI::Composition::{Compositor, Desktop::DesktopWindowTarget},
};
use windows::Interface;

pub trait CompositorDesktopInterop {
    fn create_desktop_window_target(
        &self,
        window_handle: &HWND,
        is_top_most: bool,
    ) -> windows::Result<DesktopWindowTarget>;
}

impl CompositorDesktopInterop for Compositor {
    fn create_desktop_window_target(
        &self,
        window_handle: &HWND,
        is_top_most: bool,
    ) -> windows::Result<DesktopWindowTarget> {
        let interop: ICompositorDesktopInterop = self.cast()?;
        unsafe { interop.CreateDesktopWindowTarget(window_handle, is_top_most) }
    }
}

pub trait GraphicsCaptureItemInterop {
    fn create_for_window(window_handle: &HWND) -> windows::Result<GraphicsCaptureItem>;
}

impl GraphicsCaptureItemInterop for GraphicsCaptureItem {
    fn create_for_window(window_handle: &HWND) -> windows::Result<GraphicsCaptureItem> {
        let interop = windows::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        unsafe { interop.CreateForWindow(window_handle) }
    }
}

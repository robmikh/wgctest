use windows::{
    Graphics::Capture::GraphicsCaptureItem,
    Win32::{
        Foundation::HWND,
        System::WinRT::{Composition::ICompositorDesktopInterop, Graphics::Capture::IGraphicsCaptureItemInterop},
    },
    UI::Composition::{Compositor, Desktop::DesktopWindowTarget},
};
use windows::core::Interface;

pub trait CompositorDesktopInterop {
    fn create_desktop_window_target(
        &self,
        window_handle: &HWND,
        is_top_most: bool,
    ) -> windows::core::Result<DesktopWindowTarget>;
}

impl CompositorDesktopInterop for Compositor {
    fn create_desktop_window_target(
        &self,
        window_handle: &HWND,
        is_top_most: bool,
    ) -> windows::core::Result<DesktopWindowTarget> {
        let interop: ICompositorDesktopInterop = self.cast()?;
        unsafe { interop.CreateDesktopWindowTarget(*window_handle, is_top_most) }
    }
}

pub trait GraphicsCaptureItemInterop {
    fn create_for_window(window_handle: &HWND) -> windows::core::Result<GraphicsCaptureItem>;
}

impl GraphicsCaptureItemInterop for GraphicsCaptureItem {
    fn create_for_window(window_handle: &HWND) -> windows::core::Result<GraphicsCaptureItem> {
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        unsafe { interop.CreateForWindow(*window_handle) }
    }
}

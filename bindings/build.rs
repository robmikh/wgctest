fn main() {
    windows::build!(
        Windows::Foundation::*,
        Windows::Foundation::Numerics::*,
        Windows::Graphics::Imaging::{
            BitmapEncoder, BitmapPixelFormat, BitmapAlphaMode,
        },
        Windows::Storage::{
            StorageFolder, StorageFile, CreationCollisionOption, FileAccessMode,
        },
        Windows::Storage::Streams::{
            IRandomAccessStream,
        },
        Windows::Win32::UI::HiDpi::{
            SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        },
        Windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, WNDCLASSW, LoadCursorW, IDC_ARROW, DefWindowProcW, RegisterClassW,
            CW_USEDEFAULT, AdjustWindowRectEx, ShowWindow, DestroyWindow, GetClientRect, WM_NCCREATE,
            CREATESTRUCTW, SetWindowLongW, SetWindowLongPtrW, GetWindowLongW, GetWindowLongPtrW,
        },
        Windows::Win32::System::LibraryLoader::GetModuleHandleW,
        Windows::Win32::System::SystemServices::{
            DPI_AWARENESS_CONTEXT,
        },
        Windows::Win32::Graphics::Dxgi::{
            DXGI_FORMAT,
            DXGI_SAMPLE_DESC,
            IDXGIDevice,
            DXGI_ERROR_UNSUPPORTED,
            IDXGISwapChain1,
            DXGI_SWAP_CHAIN_DESC1,
            IDXGIAdapter,
            IDXGIFactory2,
            IDXGIDevice2,
            IDXGIOutput,
            DXGI_PRESENT_PARAMETERS,
            DXGI_USAGE_RENDER_TARGET_OUTPUT,
        },
        Windows::Win32::Graphics::Direct3D11::{
            D3D11CreateDevice,
            D3D_DRIVER_TYPE,
            D3D11_CREATE_DEVICE_FLAG,
            D3D11_SDK_VERSION,
            ID3D11Device,
            D3D11_TEXTURE2D_DESC,
            D3D11_USAGE,
            D3D11_BIND_FLAG,
            D3D11_RENDER_TARGET_VIEW_DESC,
            D3D11_RTV_DIMENSION,
            ID3D11Resource,
            ID3D11RenderTargetView,
            D3D11_CPU_ACCESS_FLAG,
            D3D11_MAPPED_SUBRESOURCE,
            D3D11_MAP,
            ID3D11Texture2D,
            D3D11_SUBRESOURCE_DATA,
            ID3D11DeviceContext,
            D3D11_BOX,
            ID3D11DeviceChild,
        },
        Windows::Win32::Graphics::Dwm::{
            DwmGetWindowAttribute, DWMWINDOWATTRIBUTE,
        },
        Windows::Win32::Graphics::Gdi::ClientToScreen,
        Windows::Win32::System::WinRT::{
           RO_INIT_TYPE,
           RoInitialize,
           IGraphicsCaptureItemInterop,
           IDirect3DDxgiInterfaceAccess,
           CreateDirect3D11DeviceFromDXGIDevice,
           CreateDispatcherQueueController,
           DispatcherQueueOptions,
           ICompositorDesktopInterop,
           IGraphicsCaptureItemInterop,
        },
        Windows::System::{
            DispatcherQueueController,
            DispatcherQueue,
            DispatcherQueueHandler,
        },
        Windows::UI::{
            Color, Colors,
        },
        Windows::UI::Composition::{
            Compositor, ShapeVisual, CompositionEllipseGeometry, CompositionSpriteShape,
            CompositionColorBrush, CompositionShapeCollection, CompositionObject, SpriteVisual,
        },
        Windows::UI::Composition::Core::{
            CompositorController,
        },
        Windows::UI::Composition::Desktop::{
            DesktopWindowTarget,
        },
        Windows::Storage::{
            StorageFolder,
            StorageFile,
            CreationCollisionOption,
            FileAccessMode,
        },
        Windows::Storage::Streams::{
            IRandomAccessStream,
        },
        Windows::Graphics::{
            SizeInt32, RectInt32,
        },
        Windows::Graphics::Capture::{
            Direct3D11CaptureFramePool,
            Direct3D11CaptureFrame,
            GraphicsCaptureSession,
            GraphicsCaptureItem,
        },
        Windows::Graphics::DirectX::{
            DirectXPixelFormat
        },
        Windows::Graphics::DirectX::Direct3D11::{
            Direct3DUsage,
            Direct3DBindings,
            IDirect3DDevice,
            IDirect3DSurface,
        },
        Windows::Graphics::Imaging::{
            BitmapEncoder,
            BitmapPixelFormat,
            BitmapAlphaMode,
        },
    );
}

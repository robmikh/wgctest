use std::time::Duration;

use bindings::Windows::{
    Graphics::{Capture::GraphicsCaptureItem, DirectX::Direct3D11::IDirect3DDevice},
    System::DispatcherQueue,
    Win32::{
        Foundation::{HWND, RECT},
        Graphics::{
            Direct3D11::{
                ID3D11Device, ID3D11DeviceContext, ID3D11RenderTargetView, ID3D11Texture2D,
            },
            Dxgi::{
                IDXGIAdapter, IDXGIDevice2, IDXGIFactory2, IDXGISwapChain1, DXGI_ALPHA_MODE_IGNORE,
                DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_PRESENT_PARAMETERS, DXGI_SAMPLE_DESC,
                DXGI_SCALING_NONE, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                DXGI_USAGE_RENDER_TARGET_OUTPUT,
            },
        },
        UI::WindowsAndMessaging::GetClientRect,
    },
    UI::Color,
};
use windows::Interface;

use crate::{d3d::get_d3d_interface_from_object, interop::GraphicsCaptureItemInterop};

use super::{
    utils::{common_colors, create_test_window_on_thread, test_center_of_surface, AsyncGraphicsCapture},
    TestResult,
};

pub async fn fullscreen_transition_test(
    test_thread_queue: &DispatcherQueue,
    device: &IDirect3DDevice,
) -> TestResult<()> {
    let width = 800;
    let height = 600;
    let d3d_device: ID3D11Device = get_d3d_interface_from_object(device)?;

    // Create and setup the test window
    let window = create_test_window_on_thread(
        &test_thread_queue,
        "wgctest - Fullscreen Transition Test",
        width,
        height,
    )?;
    let mut swap_chain = TestSwapChain::new(&d3d_device, width, height, &window.0)?;
    swap_chain.flip(&common_colors::RED)?;

    async_std::task::sleep(Duration::from_millis(500)).await;

    // Start the capture
    let item = GraphicsCaptureItem::create_for_window(&window.0)?;
    let capture = AsyncGraphicsCapture::new(device, item)?;

    // The first frame should be red
    let frame = capture.get_next_frame().await?;
    test_center_of_surface(frame.Surface()?, &common_colors::RED)?;

    // Transition to fullscreen
    swap_chain.set_fullscreen(true)?;
    swap_chain.flip(&common_colors::GREEN)?;
    // Wait for the transition
    async_std::task::sleep(Duration::from_millis(500)).await;

    // Release the previous frame and get a new one
    frame.Close()?;
    let frame = capture.get_next_frame().await?;

    // Test for green
    test_center_of_surface(frame.Surface()?, &common_colors::GREEN)?;

    // Transition to windowed
    swap_chain.set_fullscreen(false)?;
    swap_chain.flip(&common_colors::BLUE)?;
    // Wait for the transition
    async_std::task::sleep(Duration::from_millis(500)).await;

    // Release the previous frame and get a new one
    frame.Close()?;
    let frame = capture.get_next_frame().await?;

    // Test for blue
    test_center_of_surface(frame.Surface()?, &common_colors::BLUE)?;

    Ok(())
}

struct TestSwapChain {
    d3d_device: ID3D11Device,
    d3d_context: ID3D11DeviceContext,
    swap_chain: IDXGISwapChain1,
    render_target_view: Option<ID3D11RenderTargetView>,
    window: HWND,
    is_fullscreen: bool,
}

impl TestSwapChain {
    pub fn new(
        d3d_device: &ID3D11Device,
        width: u32,
        height: u32,
        window: &HWND,
    ) -> windows::Result<Self> {
        let mut d3d_context = None;
        unsafe { d3d_device.GetImmediateContext(&mut d3d_context) };
        let d3d_context = d3d_context.unwrap();

        let desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: width,
            Height: height,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BufferCount: 2,
            Scaling: DXGI_SCALING_NONE,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode: DXGI_ALPHA_MODE_IGNORE,
            Stereo: false.into(),
            Flags: 0,
        };

        let dxgi_device: IDXGIDevice2 = d3d_device.cast()?;
        let adapter: IDXGIAdapter = unsafe { dxgi_device.GetParent()? };
        let factory: IDXGIFactory2 = unsafe { adapter.GetParent()? };

        let swap_chain = unsafe {
            factory.CreateSwapChainForHwnd(d3d_device, window, &desc, std::ptr::null(), None)?
        };

        let back_buffer: ID3D11Texture2D = unsafe { swap_chain.GetBuffer(0)? };
        let render_target_view =
            unsafe { d3d_device.CreateRenderTargetView(&back_buffer, std::ptr::null())? };

        Ok(Self {
            d3d_device: d3d_device.clone(),
            d3d_context,
            swap_chain,
            render_target_view: Some(render_target_view),
            window: *window,
            is_fullscreen: false,
        })
    }

    pub fn set_fullscreen(&mut self, fullscreen: bool) -> windows::Result<()> {
        if fullscreen != self.is_fullscreen {
            self.is_fullscreen = fullscreen;
            let (width, height) = if fullscreen {
                let dxgi_device: IDXGIDevice2 = self.d3d_device.cast()?;
                let adapter: IDXGIAdapter = unsafe { dxgi_device.GetParent()? };
                let output = unsafe { adapter.EnumOutputs(0)? };
                let output_desc = unsafe { output.GetDesc()? };

                let width =
                    output_desc.DesktopCoordinates.right - output_desc.DesktopCoordinates.left;
                let height =
                    output_desc.DesktopCoordinates.bottom - output_desc.DesktopCoordinates.top;

                unsafe { self.swap_chain.SetFullscreenState(true, &output)? };

                (width as u32, height as u32)
            } else {
                let mut rect = RECT::default();
                unsafe { GetClientRect(&self.window, &mut rect).ok()? };

                let width = rect.right - rect.left;
                let height = rect.bottom - rect.top;

                unsafe { self.swap_chain.SetFullscreenState(false, None)? };

                (width as u32, height as u32)
            };
            self.render_target_view = None;
            unsafe {
                self.swap_chain
                    .ResizeBuffers(2, width, height, DXGI_FORMAT_B8G8R8A8_UNORM, 0)?
            };
            let back_buffer: ID3D11Texture2D = unsafe { self.swap_chain.GetBuffer(0)? };
            let render_target_view = unsafe {
                self.d3d_device
                    .CreateRenderTargetView(&back_buffer, std::ptr::null())?
            };
            self.render_target_view = Some(render_target_view);
        }
        Ok(())
    }

    pub fn flip(&self, color: &Color) -> windows::Result<()> {
        let color_f = [
            color.R as f32 / 255.0,
            color.G as f32 / 255.0,
            color.B as f32 / 255.0,
            color.A as f32 / 255.0,
        ];
        unsafe {
            self.d3d_context
                .ClearRenderTargetView(&self.render_target_view, &color_f as *const _)
        };
        let present_params = DXGI_PRESENT_PARAMETERS::default();
        unsafe { self.swap_chain.Present1(0, 0, &present_params)? };
        Ok(())
    }
}

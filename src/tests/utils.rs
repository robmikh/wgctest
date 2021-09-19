use std::sync::mpsc::channel;

use bindings::Windows::Foundation::TypedEventHandler;
use bindings::Windows::Graphics::Capture::{
    Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use bindings::Windows::Graphics::DirectX::Direct3D11::{IDirect3DDevice, IDirect3DSurface};
use bindings::Windows::Graphics::DirectX::DirectXPixelFormat;
use bindings::Windows::System::{DispatcherQueue, DispatcherQueueHandler};
use bindings::Windows::Win32::Graphics::Direct3D11::ID3D11DeviceChild;
use bindings::Windows::Win32::Graphics::Dxgi::DXGI_FORMAT_B8G8R8A8_UNORM;
use bindings::Windows::{
    Win32::Graphics::Direct3D11::{
        ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D, D3D11_MAPPED_SUBRESOURCE,
        D3D11_MAP_READ, D3D11_TEXTURE2D_DESC,
    },
    UI::Color,
};
use windows::Interface;

use crate::d3d::{copy_texture, get_d3d_interface_from_object};
use crate::test_window::{create_test_window, TestWindow};

use super::{TestError, TestResult, TextureError};

pub struct MappedTexture<'a> {
    d3d_context: ID3D11DeviceContext,
    texture: &'a ID3D11Texture2D,
    texture_desc: D3D11_TEXTURE2D_DESC,
    mapped_data: D3D11_MAPPED_SUBRESOURCE,
}

impl<'a> MappedTexture<'a> {
    pub fn new(texture: &'a ID3D11Texture2D) -> windows::Result<Self> {
        let d3d_context = unsafe {
            let mut d3d_device = None;
            texture.GetDevice(&mut d3d_device);
            let d3d_device = d3d_device.unwrap();

            let mut d3d_context = None;
            d3d_device.GetImmediateContext(&mut d3d_context);
            d3d_context.unwrap()
        };
        let texture_desc = unsafe {
            let mut texture_desc = D3D11_TEXTURE2D_DESC::default();
            texture.GetDesc(&mut texture_desc);
            texture_desc
        };
        // TODO: Support other pixel formats
        assert_eq!(texture_desc.Format, DXGI_FORMAT_B8G8R8A8_UNORM);
        let resource: ID3D11Resource = texture.cast()?;
        let mapped_data = unsafe { d3d_context.Map(Some(resource.clone()), 0, D3D11_MAP_READ, 0)? };

        Ok(Self {
            d3d_context,
            texture,
            texture_desc,
            mapped_data,
        })
    }

    pub fn read_pixel(&self, x: u32, y: u32) -> Option<Color> {
        if x < self.texture_desc.Width && y < self.texture_desc.Height {
            let bytes_per_pixel = 4;
            // Get a slice of bytes
            let data: &[u8] = unsafe {
                std::slice::from_raw_parts(
                    self.mapped_data.pData as *const _,
                    (self.texture_desc.Height * self.mapped_data.RowPitch) as usize,
                )
            };
            let offset = ((self.mapped_data.RowPitch * y) + (x * bytes_per_pixel)) as usize;
            let b = data[offset + 0];
            let g = data[offset + 1];
            let r = data[offset + 2];
            let a = data[offset + 3];
            Some(Color {
                B: b,
                G: g,
                R: r,
                A: a,
            })
        } else {
            None
        }
    }
}

impl<'a> Drop for MappedTexture<'a> {
    fn drop(&mut self) {
        unsafe { self.d3d_context.Unmap(self.texture, 0) };
    }
}

pub enum ColorCheck {
    Success,
    Different(String),
}

impl ColorCheck {
    pub fn ok(self, texture: &ID3D11Texture2D) -> TestResult<()> {
        match self {
            ColorCheck::Success => Ok(()),
            ColorCheck::Different(message) => TestError::Texture(TextureError {
                message,
                texture: texture.clone(),
            })
            .ok(),
        }
    }
}

pub fn check_color(actual: Color, expected: Color) -> ColorCheck {
    if actual != expected {
        ColorCheck::Different(format!(
            r#"Color comparison failed!
  Actual: ( B: {}, G: {}, R: {}, A: {} )
  Expected: ( B: {}, G: {}, R: {}, A: {} )
"#,
            actual.B, actual.G, actual.R, actual.A, expected.B, expected.G, expected.R, expected.A
        ))
    } else {
        ColorCheck::Success
    }
}

pub fn create_test_window_on_thread(
    dispatcher_queue: &DispatcherQueue,
    title: &'static str,
    width: u32,
    height: u32,
) -> windows::Result<TestWindow> {
    let (sender, receiver) = channel();
    dispatcher_queue.TryEnqueue(DispatcherQueueHandler::new(
        move || -> windows::Result<()> {
            let window = create_test_window(title, width, height)?;
            sender.send(window).unwrap();
            Ok(())
        },
    ))?;
    let window = receiver.recv().unwrap();
    Ok(window)
}

pub struct AsyncGraphicsCapture {
    _item: GraphicsCaptureItem,
    frame_pool: Direct3D11CaptureFramePool,
    session: GraphicsCaptureSession,
    receiver: async_std::channel::Receiver<Direct3D11CaptureFrame>,
}

impl AsyncGraphicsCapture {
    pub fn new(device: &IDirect3DDevice, item: GraphicsCaptureItem) -> windows::Result<Self> {
        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            1,
            item.Size()?,
        )?;
        let (sender, receiver) = async_std::channel::bounded(1);
        frame_pool.FrameArrived(TypedEventHandler::<
            Direct3D11CaptureFramePool,
            windows::IInspectable,
        >::new(
            move |frame_pool, _| -> windows::Result<()> {
                let frame_pool = frame_pool.as_ref().unwrap();
                let frame = frame_pool.TryGetNextFrame()?;
                async_std::task::block_on(sender.send(frame)).unwrap();
                Ok(())
            },
        ))?;
        let session = frame_pool.CreateCaptureSession(&item)?;
        session.StartCapture()?;
        Ok(Self {
            _item: item,
            frame_pool,
            session,
            receiver,
        })
    }

    pub async fn get_next_frame(&self) -> windows::Result<Direct3D11CaptureFrame> {
        let frame = self.receiver.recv().await.unwrap();
        Ok(frame)
    }
}

impl Drop for AsyncGraphicsCapture {
    fn drop(&mut self) {
        self.session.Close().unwrap();
        self.frame_pool.Close().unwrap();
    }
}

pub fn test_center_of_surface(frame: IDirect3DSurface, color: &Color) -> TestResult<()> {
    let description = frame.Description()?;
    let x = description.Width / 2;
    let y = description.Height / 2;
    test_surface_at_point(frame, color, x as u32, y as u32)
}

pub fn test_surface_at_point(
    frame: IDirect3DSurface,
    color: &Color,
    x: u32,
    y: u32,
) -> TestResult<()> {
    let texture: ID3D11Texture2D = get_d3d_interface_from_object(&frame)?;
    let d3d_device = {
        let mut d3d_device = None;
        let child: ID3D11DeviceChild = texture.cast()?;
        unsafe { child.GetDevice(&mut d3d_device) };
        d3d_device.unwrap()
    };
    let d3d_context = {
        let mut d3d_context = None;
        unsafe { d3d_device.GetImmediateContext(&mut d3d_context) };
        d3d_context.unwrap()
    };
    let new_texture = copy_texture(&d3d_device, &d3d_context, &texture, true)?;
    {
        let mapped = MappedTexture::new(&new_texture)?;
        check_color(mapped.read_pixel(x, y).unwrap(), *color).ok(&new_texture)
    }
}

pub mod common_colors {
    use bindings::Windows::UI::Color;

    pub const TRANSPARENT_BLACK: Color = Color {
        A: 0,
        R: 0,
        G: 0,
        B: 0,
    };
    pub const RED: Color = Color {
        A: 255,
        R: 255,
        G: 0,
        B: 0,
    };
    pub const BLUE: Color = Color {
        A: 255,
        R: 0,
        G: 0,
        B: 255,
    };
    pub const GREEN: Color = Color {
        A: 255,
        R: 0,
        G: 255,
        B: 0,
    };
}

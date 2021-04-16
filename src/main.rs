mod d3d;
mod dispatcher_queue;
mod snapshot;

use std::sync::mpsc::channel;

use bindings::Windows::Win32::Dxgi::DXGI_FORMAT;
use bindings::Windows::Win32::HiDpi::SetProcessDpiAwarenessContext;
use bindings::Windows::Win32::SystemServices::DPI_AWARENESS_CONTEXT;
use bindings::Windows::{
    Foundation::Numerics::Vector2,
    Graphics::{
        Capture::GraphicsCaptureItem,
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
    },
    System::{DispatcherQueueController, DispatcherQueueHandler},
    Win32::Direct3D11::{
        ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_MAP, D3D11_MAPPED_SUBRESOURCE,
        D3D11_TEXTURE2D_DESC,
    },
    UI::{
        Color, Colors,
        Composition::{Compositor, Core::CompositorController},
    },
};
use d3d::{create_d3d_device, create_direct3d_device, get_d3d_interface_from_object};
use snapshot::take_snapshot;
use windows::{Error, HRESULT};

macro_rules! run_test {
    ($test_name:ident, $($param:tt)*) => {
        {
            let success = $test_name($($param)*).await?;
            let message = match success {
                true => "PASSED",
                false => "FAILED",
            };
            println!("{}: {}", stringify!($test_name), message);
        }
    }
}

struct MappedTexture<'a> {
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
        assert_eq!(texture_desc.Format, DXGI_FORMAT::DXGI_FORMAT_B8G8R8A8_UNORM);
        let mapped_data = unsafe {
            let mut mapped_data = D3D11_MAPPED_SUBRESOURCE::default();
            d3d_context
                .Map(texture, 0, D3D11_MAP::D3D11_MAP_READ, 0, &mut mapped_data)
                .ok()?;
            mapped_data
        };

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

fn check_color(actual: Color, expected: Color) -> bool {
    if actual != expected {
        println!(
            r#"Color comparison failed!
  Actual: ( B: {}, G: {}, R: {}, A: {} )
  Expected: ( B: {}, G: {}, R: {}, A: {} )
"#,
            actual.B, actual.G, actual.R, actual.A, expected.B, expected.G, expected.R, expected.A
        );
        false
    } else {
        true
    }
}

async fn alpha_test(
    compositor_controller: &CompositorController,
    device: &IDirect3DDevice,
) -> windows::Result<bool> {
    let compositor = compositor_controller.Compositor()?;

    let mut success = true;

    // Build the visual tree
    // A red circle centered in a 100 x 100 bitmap with a transparent background.
    let visual = compositor.CreateShapeVisual()?;
    visual.SetSize(Vector2::new(100.0, 100.0))?;
    let geometry = compositor.CreateEllipseGeometry()?;
    geometry.SetCenter(Vector2::new(50.0, 50.0))?;
    geometry.SetRadius(Vector2::new(50.0, 50.0))?;
    let shape = compositor.CreateSpriteShapeWithGeometry(geometry)?;
    shape.SetFillBrush(compositor.CreateColorBrushWithColor(Colors::Red()?)?)?;
    visual.Shapes()?.Append(shape)?;

    // Capture the tree
    let item = GraphicsCaptureItem::CreateFromVisual(&visual)?;
    let frame = take_snapshot(
        device,
        &item,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        true,
        true,
        || {
            // We need to commit after the capture is started
            compositor_controller.Commit()
        },
    )
    .await?;

    // Map the texture and check the image
    {
        let mapped = MappedTexture::new(&frame)?;

        if !check_color(mapped.read_pixel(50, 50).unwrap(), Colors::Red()?) {
            success = false;
        }
        // We don't use Colors::Transparent() here becuase that is transparent white.
        // Right now the capture API uses transparent black to clear.
        if !check_color(
            mapped.read_pixel(5, 5).unwrap(),
            Color {
                B: 0,
                G: 0,
                R: 0,
                A: 0,
            },
        ) {
            success = false;
        }
    }

    Ok(success)
}

#[async_std::main]
async fn main() -> windows::Result<()> {
    // NOTE: We don't properly scale any of the UI or properly respond to DPI changes, but none of
    //       the UI is meant to be interacted with. This is just so that the tests don't get
    //       virtualized coordinates on high DPI machines.
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT(-4)) };
    windows::initialize_sta()?;

    // The compositor needs a DispatcherQueue. We'll create one on a dedicated thread so that
    // we can block the main thread if we need to.
    let dispatcher_controller = DispatcherQueueController::CreateOnDedicatedThread()?;
    let compositor_queue = dispatcher_controller.DispatcherQueue()?;
    // Because the tests themselves won't be running on the compositor thread, we'll need to
    // controll when our changes are committed. Create a CompositorController so we have control
    // over calling Commit.
    let compositor_controller = {
        let (sender, receiver) = channel();
        compositor_queue.TryEnqueue(DispatcherQueueHandler::new(
            move || -> windows::Result<()> {
                let compositor_controller = CompositorController::new()?;
                sender.send(compositor_controller).unwrap();
                Ok(())
            },
        ))?;
        receiver.recv().unwrap()
    };
    let compositor = compositor_controller.Compositor()?;

    // Initialize D3D
    let d3d_device = create_d3d_device()?;
    let device = create_direct3d_device(&d3d_device)?;

    // Run tests
    // TODO: Allow filters to only run certain tests
    run_test!(alpha_test, &compositor_controller, &device);

    Ok(())
}

mod d3d;
mod interop;
mod snapshot;
mod test_window;
mod tests;
mod wide_string;

use std::sync::mpsc::channel;

use bindings::Windows::Graphics::Imaging::{BitmapAlphaMode, BitmapEncoder, BitmapPixelFormat};
use bindings::Windows::Storage::{CreationCollisionOption, FileAccessMode, StorageFolder};
use bindings::Windows::Win32::Graphics::Direct3D11::{
    ID3D11DeviceChild, ID3D11Resource, ID3D11Texture2D, D3D11_MAP_READ, D3D11_TEXTURE2D_DESC,
};
use bindings::Windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
use bindings::Windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use bindings::Windows::{
    System::{DispatcherQueueController, DispatcherQueueHandler},
    UI::Composition::Core::CompositorController,
};
use d3d::{create_d3d_device, create_direct3d_device};
use windows::Interface;

use crate::tests::{alpha_test, basic_window_test, fullscreen_transition_test};

macro_rules! run_test {
    ($test_name:ident, $($param:tt)*) => {
        {
            let result = $test_name($($param)*).await;
            let status = match result {
                Ok(_) => "PASSED".to_owned(),
                Err(error) => {
                    if let crate::tests::TestError::Texture(texture_error) = &error {
                        save_image_async(stringify!($test_name), &texture_error.texture).await?;
                    }
                    format!("FAILED - {}", error)
                },
            };
            println!("{}: {}", stringify!($test_name), status);
        }
    }
}

#[async_std::main]
async fn main() -> windows::Result<()> {
    // NOTE: We don't properly scale any of the UI or properly respond to DPI changes, but none of
    //       the UI is meant to be interacted with. This is just so that the tests don't get
    //       virtualized coordinates on high DPI machines.
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    unsafe { RoInitialize(RO_INIT_MULTITHREADED)? };

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

    // Initialize D3D
    let d3d_device = create_d3d_device()?;
    let device = create_direct3d_device(&d3d_device)?;

    // Run tests
    // TODO: Allow filters to only run certain tests
    run_test!(alpha_test, &compositor_controller, &device);
    run_test!(
        basic_window_test,
        &compositor_queue,
        &compositor_controller,
        &device
    );
    run_test!(fullscreen_transition_test, &compositor_queue, &device);

    Ok(())
}

async fn save_image_async(file_stem: &str, texture: &ID3D11Texture2D) -> windows::Result<()> {
    let path = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let folder = StorageFolder::GetFolderFromPathAsync(path.as_str())?.await?;
    let file = folder
        .CreateFileAsync(
            format!("{}.png", file_stem),
            CreationCollisionOption::ReplaceExisting,
        )?
        .await?;

    let child: ID3D11DeviceChild = texture.cast()?;
    let d3d_device = {
        let mut d3d_device = None;
        unsafe { child.GetDevice(&mut d3d_device) };
        d3d_device.unwrap()
    };
    let d3d_context = {
        let mut d3d_context = None;
        unsafe { d3d_device.GetImmediateContext(&mut d3d_context) };
        d3d_context.unwrap()
    };
    let (bytes, width, height) = unsafe {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        texture.GetDesc(&mut desc as *mut _);

        let resource: ID3D11Resource = texture.cast()?;
        let mapped = d3d_context.Map(Some(resource.clone()), 0, D3D11_MAP_READ, 0)?;

        // Get a slice of bytes
        let slice: &[u8] = {
            std::slice::from_raw_parts(
                mapped.pData as *const _,
                (desc.Height * mapped.RowPitch) as usize,
            )
        };

        let bytes_per_pixel = 4;
        let mut bytes = vec![0u8; (desc.Width * desc.Height * bytes_per_pixel) as usize];
        for row in 0..desc.Height {
            let data_begin = (row * (desc.Width * bytes_per_pixel)) as usize;
            let data_end = ((row + 1) * (desc.Width * bytes_per_pixel)) as usize;
            let slice_begin = (row * mapped.RowPitch) as usize;
            let slice_end = slice_begin + (desc.Width * bytes_per_pixel) as usize;
            bytes[data_begin..data_end].copy_from_slice(&slice[slice_begin..slice_end]);
        }

        d3d_context.Unmap(Some(resource), 0);

        (bytes, desc.Width, desc.Height)
    };

    {
        let stream = file.OpenAsync(FileAccessMode::ReadWrite)?.await?;
        let encoder = BitmapEncoder::CreateAsync(BitmapEncoder::PngEncoderId()?, stream)?.await?;
        encoder.SetPixelData(
            BitmapPixelFormat::Bgra8,
            BitmapAlphaMode::Premultiplied,
            width,
            height,
            1.0,
            1.0,
            &bytes,
        )?;
        encoder.FlushAsync()?.await?;
    }

    Ok(())
}

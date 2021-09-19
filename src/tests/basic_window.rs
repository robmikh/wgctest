use bindings::Windows::{
    Foundation::Numerics::Vector2,
    Graphics::DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
    System::DispatcherQueue,
    UI::Composition::Core::CompositorController,
};

use crate::util::{
    color::{check_color, common_colors},
    error::TestResult,
    interop::CompositorDesktopInterop,
    mapped::MappedTexture,
    snapshot::take_snapshot_of_client_area,
    test_window::TestWindow,
};

pub async fn basic_window_test(
    test_thread_queue: &DispatcherQueue,
    compositor_controller: &CompositorController,
    device: &IDirect3DDevice,
) -> TestResult<()> {
    let width = 500;
    let height = 500;

    // Create and setup the test window
    let window = TestWindow::new_on_thread(
        &test_thread_queue,
        "wgctest - Basic Window Test",
        width,
        height,
    )?;
    let compositor = compositor_controller.Compositor()?;
    let target = compositor.create_desktop_window_target(&window.handle(), false)?;
    let root = compositor.CreateSpriteVisual()?;
    root.SetRelativeSizeAdjustment(Vector2::new(1.0, 1.0))?;
    root.SetBrush(compositor.CreateColorBrushWithColor(common_colors::GREEN)?)?;
    target.SetRoot(root)?;
    compositor_controller.Commit()?;

    // Capture the window
    let frame = take_snapshot_of_client_area(
        device,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        true,
        true,
        &window.handle(),
    )
    .await?;

    // Map the texture and check the image
    {
        let mapped = MappedTexture::new(&frame)?;
        check_color(
            mapped.read_pixel(width / 2, height / 2).unwrap(),
            common_colors::GREEN,
        )
        .ok(&frame)?;
    }

    Ok(())
}

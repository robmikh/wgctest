use super::utils::{MappedTexture, check_color, common_colors};
use crate::snapshot::take_snapshot;
use bindings::Windows::{
    Foundation::Numerics::Vector2,
    Graphics::{
        Capture::GraphicsCaptureItem,
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
    },
    UI::Composition::Core::CompositorController,
};

pub async fn alpha_test(
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
    shape.SetFillBrush(compositor.CreateColorBrushWithColor(&common_colors::RED)?)?;
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

        if !check_color(mapped.read_pixel(50, 50).unwrap(), common_colors::RED) {
            success = false;
        }
        if !check_color(
            mapped.read_pixel(5, 5).unwrap(),
            common_colors::TRANSPARENT_BLACK,
        ) {
            success = false;
        }
    }

    Ok(success)
}

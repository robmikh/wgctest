use async_std::{channel::bounded, task::block_on};
use bindings::Windows::{
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem},
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
    },
    Win32::Graphics::Direct3D11::{
        ID3D11Device, ID3D11Texture2D, D3D11_BIND_FLAG, D3D11_BIND_SHADER_RESOURCE,
        D3D11_CPU_ACCESS_FLAG, D3D11_CPU_ACCESS_READ, D3D11_RESOURCE_MISC_FLAG,
        D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING,
    },
};
use windows::Interface;

use crate::d3d::get_d3d_interface_from_object;

pub async fn take_snapshot<F: Fn() -> windows::Result<()>>(
    device: &IDirect3DDevice,
    item: &GraphicsCaptureItem,
    pixel_format: DirectXPixelFormat,
    staging_texture: bool,
    cursor_enabled: bool,
    started: F,
) -> windows::Result<ID3D11Texture2D> {
    let item_size = item.Size()?;

    let d3d_device: ID3D11Device = get_d3d_interface_from_object(device)?;
    let d3d_context = unsafe {
        let mut d3d_context = None;
        d3d_device.GetImmediateContext(&mut d3d_context);
        d3d_context.unwrap()
    };

    let frame_pool =
        Direct3D11CaptureFramePool::CreateFreeThreaded(device, pixel_format, 1, item_size)?;
    let session = frame_pool.CreateCaptureSession(item)?;
    if !cursor_enabled {
        session.SetIsCursorCaptureEnabled(false)?;
    }

    let (sender, receiver) = bounded(1);
    frame_pool.FrameArrived(TypedEventHandler::<
        Direct3D11CaptureFramePool,
        windows::IInspectable,
    >::new(move |frame_pool, _| {
        let frame_pool = frame_pool.as_ref().unwrap();
        let frame = frame_pool.TryGetNextFrame()?;
        block_on(sender.send(frame)).unwrap();
        Ok(())
    }))?;
    session.StartCapture()?;
    started()?;

    let frame = receiver.recv().await.unwrap();
    let result_texture = unsafe {
        let source_texture: ID3D11Texture2D = get_d3d_interface_from_object(&frame.Surface()?)?;
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        source_texture.GetDesc(&mut desc);
        desc.MiscFlags = D3D11_RESOURCE_MISC_FLAG(0);
        if staging_texture {
            desc.Usage = D3D11_USAGE_STAGING;
            desc.BindFlags = D3D11_BIND_FLAG(0);
            desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
        } else {
            desc.Usage = D3D11_USAGE_DEFAULT;
            desc.BindFlags = D3D11_BIND_SHADER_RESOURCE;
            desc.CPUAccessFlags = D3D11_CPU_ACCESS_FLAG(0);
        }
        let texture = d3d_device.CreateTexture2D(&desc, std::ptr::null())?;
        d3d_context.CopyResource(Some(texture.cast()?), Some(source_texture.cast()?));

        texture
    };

    session.Close()?;
    frame_pool.Close()?;
    frame.Close()?;

    Ok(result_texture)
}

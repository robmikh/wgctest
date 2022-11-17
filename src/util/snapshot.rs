use async_std::{channel::bounded, task::block_on};
use windows::{
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem},
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
        RectInt32,
    },
    Win32::{
        Foundation::{HWND, POINT, RECT},
        Graphics::{
            Direct3D11::{
                ID3D11Device, ID3D11Texture2D, D3D11_BIND_FLAG, D3D11_BIND_SHADER_RESOURCE,
                D3D11_BOX, D3D11_CPU_ACCESS_FLAG, D3D11_CPU_ACCESS_READ, D3D11_RESOURCE_MISC_FLAG,
                D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING, ID3D11Resource,
            },
            Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS},
            Gdi::ClientToScreen,
        },
        UI::WindowsAndMessaging::GetClientRect,
    },
    UI::Composition::Core::CompositorController,
};
use windows::core::Interface;

use super::{d3d::get_d3d_interface_from_object, interop::GraphicsCaptureItemInterop};

pub async fn take_snapshot(
    device: &IDirect3DDevice,
    item: &GraphicsCaptureItem,
    pixel_format: DirectXPixelFormat,
    staging_texture: bool,
    cursor_enabled: bool,
) -> windows::core::Result<ID3D11Texture2D> {
    let texture = take_snapshot_internal(
        device,
        item,
        pixel_format,
        staging_texture,
        cursor_enabled,
        None,
        || -> windows::core::Result<()> { Ok(()) },
    )
    .await?;
    Ok(texture)
}

pub async fn take_snapshot_with_commit(
    device: &IDirect3DDevice,
    item: &GraphicsCaptureItem,
    pixel_format: DirectXPixelFormat,
    staging_texture: bool,
    cursor_enabled: bool,
    compositor_controller: &CompositorController,
) -> windows::core::Result<ID3D11Texture2D> {
    let texture = take_snapshot_internal(
        device,
        item,
        pixel_format,
        staging_texture,
        cursor_enabled,
        None,
        || -> windows::core::Result<()> { compositor_controller.Commit() },
    )
    .await?;
    Ok(texture)
}

pub async fn take_snapshot_of_client_area(
    device: &IDirect3DDevice,
    pixel_format: DirectXPixelFormat,
    staging_texture: bool,
    cursor_enabled: bool,
    window_handle: &HWND,
) -> windows::core::Result<ID3D11Texture2D> {
    let mut client_rect = RECT::default();
    unsafe { GetClientRect(*window_handle, &mut client_rect).ok()? };

    let mut client_origin = POINT {
        x: client_rect.left,
        y: client_rect.top,
    };
    unsafe { ClientToScreen(*window_handle, &mut client_origin).ok()? };

    let mut window_bounds = RECT::default();
    unsafe {
        DwmGetWindowAttribute(
            *window_handle,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut window_bounds as *mut _ as *mut _,
            std::mem::size_of::<RECT>() as u32,
        )?
    };

    let rect = RectInt32 {
        X: client_origin.x - window_bounds.left,
        Y: client_origin.y - window_bounds.top,
        Width: client_rect.right - client_rect.left,
        Height: client_rect.bottom - client_rect.top,
    };

    let item = GraphicsCaptureItem::create_for_window(window_handle)?;
    let texture = take_snapshot_internal(
        device,
        &item,
        pixel_format,
        staging_texture,
        cursor_enabled,
        Some(rect),
        || -> windows::core::Result<()> { Ok(()) },
    )
    .await?;
    Ok(texture)
}

async fn take_snapshot_internal<F: Fn() -> windows::core::Result<()>>(
    device: &IDirect3DDevice,
    item: &GraphicsCaptureItem,
    pixel_format: DirectXPixelFormat,
    staging_texture: bool,
    cursor_enabled: bool,
    rect: Option<RectInt32>,
    started: F,
) -> windows::core::Result<ID3D11Texture2D> {
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
    let handler = TypedEventHandler::<
    Direct3D11CaptureFramePool,
    windows::core::IInspectable,
>::new(move |frame_pool, _| {
    let frame_pool = frame_pool.as_ref().unwrap();
    let frame = frame_pool.TryGetNextFrame()?;
    block_on(sender.send(frame)).unwrap();
    Ok(())
});
    frame_pool.FrameArrived(&handler)?;
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
        if let Some(rect) = &rect {
            desc.Width = rect.Width as u32;
            desc.Height = rect.Height as u32;
        }
        let texture = d3d_device.CreateTexture2D(&desc, None)?;
        let resource: ID3D11Resource = texture.cast()?;
        let source_resource: ID3D11Resource = source_texture.cast()?;
        if let Some(rect) = &rect {
            let d3d_box = D3D11_BOX {
                left: rect.X as u32,
                top: rect.Y as u32,
                front: 0,
                right: (rect.X + rect.Width) as u32,
                bottom: (rect.Y + rect.Height) as u32,
                back: 1,
            };
            d3d_context.CopySubresourceRegion(
                &resource,
                0,
                0,
                0,
                0,
                &source_resource,
                0,
                Some(&d3d_box),
            );
        } else {
            d3d_context.CopyResource(&resource, &source_resource);
        }

        texture
    };

    session.Close()?;
    frame_pool.Close()?;
    frame.Close()?;

    Ok(result_texture)
}

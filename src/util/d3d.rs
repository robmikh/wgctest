use bindings::Windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use bindings::Windows::Win32::Graphics::Direct3D11::{
    ID3D11DeviceContext, ID3D11Texture2D, D3D11_BIND_FLAG, D3D11_BIND_SHADER_RESOURCE,
    D3D11_CPU_ACCESS_FLAG, D3D11_CPU_ACCESS_READ, D3D11_RESOURCE_MISC_FLAG, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING,
};
use bindings::Windows::Win32::Graphics::{
    Direct3D11::{
        D3D11CreateDevice, ID3D11Device, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION, D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE,
        D3D_DRIVER_TYPE_WARP,
    },
    Dxgi::{IDXGIDevice, DXGI_ERROR_UNSUPPORTED},
};
use bindings::Windows::Win32::System::WinRT::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};
use windows::{Abi, Interface};

fn create_d3d_device_with_type(
    driver_type: D3D_DRIVER_TYPE,
    flags: D3D11_CREATE_DEVICE_FLAG,
    device: *mut Option<ID3D11Device>,
) -> windows::Result<()> {
    unsafe {
        D3D11CreateDevice(
            None,
            driver_type,
            None,
            flags,
            std::ptr::null(),
            0,
            D3D11_SDK_VERSION as u32,
            device,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    }
}

pub fn create_d3d_device() -> windows::Result<ID3D11Device> {
    let mut device = None;
    let mut result = create_d3d_device_with_type(
        D3D_DRIVER_TYPE_HARDWARE,
        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        &mut device,
    );
    if let Err(error) = &result {
        if error.code() == DXGI_ERROR_UNSUPPORTED {
            result = create_d3d_device_with_type(
                D3D_DRIVER_TYPE_WARP,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                &mut device,
            );
        }
    }
    result?;
    Ok(device.unwrap())
}

pub fn create_direct3d_device(d3d_device: &ID3D11Device) -> windows::Result<IDirect3DDevice> {
    let dxgi_device: IDXGIDevice = d3d_device.cast()?;
    let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(Some(dxgi_device))? };
    inspectable.cast()
}

pub fn get_d3d_interface_from_object<S: Interface, R: Interface + Abi>(
    object: &S,
) -> windows::Result<R> {
    let access: IDirect3DDxgiInterfaceAccess = object.cast()?;
    let object = unsafe { access.GetInterface::<R>()? };
    Ok(object)
}

pub fn copy_texture(
    d3d_device: &ID3D11Device,
    d3d_context: &ID3D11DeviceContext,
    texture: &ID3D11Texture2D,
    staging_texture: bool,
) -> windows::Result<ID3D11Texture2D> {
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    unsafe { texture.GetDesc(&mut desc) };
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
    let new_texture = unsafe { d3d_device.CreateTexture2D(&desc, std::ptr::null())? };
    unsafe { d3d_context.CopyResource(Some(new_texture.cast()?), Some(texture.cast()?)) };
    Ok(new_texture)
}

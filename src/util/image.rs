use bindings::Windows::{
    Graphics::Imaging::{BitmapAlphaMode, BitmapEncoder, BitmapPixelFormat},
    Storage::{CreationCollisionOption, FileAccessMode, StorageFolder},
    Win32::Graphics::Direct3D11::{
        ID3D11DeviceChild, ID3D11Resource, ID3D11Texture2D, D3D11_MAP_READ, D3D11_TEXTURE2D_DESC,
    },
};
use windows::Interface;

pub async fn save_image_async(file_stem: &str, texture: &ID3D11Texture2D) -> windows::Result<()> {
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

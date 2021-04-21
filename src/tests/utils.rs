use bindings::Windows::Win32::Dxgi::DXGI_FORMAT;
use bindings::Windows::{
    Win32::Direct3D11::{
        ID3D11DeviceContext, ID3D11Texture2D, D3D11_MAP, D3D11_MAPPED_SUBRESOURCE,
        D3D11_TEXTURE2D_DESC,
    },
    UI::{
        Color,
    },
};

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

pub fn check_color(actual: Color, expected: Color) -> bool {
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
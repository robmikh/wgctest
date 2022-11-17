use windows::{
    Graphics::DirectX::Direct3D11::IDirect3DSurface,
    Win32::Graphics::Direct3D11::{ID3D11DeviceChild, ID3D11Texture2D},
    UI::Color,
};
use windows::core::Interface;

use super::{
    d3d::{copy_texture, get_d3d_interface_from_object},
    error::{TestError, TestResult, TextureError},
    mapped::MappedTexture,
};

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
    use windows::UI::Color;

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

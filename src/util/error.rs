use std::{error::Error, fmt};

use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;

pub type TestResult<T> = std::result::Result<T, TestError>;

#[derive(Debug)]
pub enum TestError {
    General(windows::core::Error),
    Texture(TextureError),
}

#[derive(Debug)]
pub struct TextureError {
    pub message: String,
    pub texture: ID3D11Texture2D,
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestError::General(error) => write!(f, "{}", error.message()),
            TestError::Texture(error) => write!(f, "{}", error.message),
        }
    }
}

impl Error for TestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            TestError::General(error) => Some(error),
            TestError::Texture(error) => Some(error),
        }
    }
}

impl From<windows::core::Error> for TestError {
    fn from(error: windows::core::Error) -> Self {
        TestError::General(error)
    }
}

impl TestError {
    pub fn ok(self) -> TestResult<()> {
        TestResult::Err(self)
    }
}

impl fmt::Display for TextureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for TextureError {}

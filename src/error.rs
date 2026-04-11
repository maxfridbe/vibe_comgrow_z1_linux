use std::fmt;
use std::error::Error;

#[derive(Debug)]
pub enum TrogdorError {
    Io(std::io::Error),
    ParseFloat(std::num::ParseFloatError),
    Json(serde_json::Error),
    Image(image::ImageError),
    Serial(serialport::Error),
    Env(std::env::VarError),
    PicoArgs(pico_args::Error),
    Usvg(usvg::Error),
    Generic(String),
}

pub type Result<T> = std::result::Result<T, TrogdorError>;

impl fmt::Display for TrogdorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrogdorError::Io(e) => write!(f, "IO Error: {}", e),
            TrogdorError::ParseFloat(e) => write!(f, "Parse Float Error: {}", e),
            TrogdorError::Json(e) => write!(f, "JSON Error: {}", e),
            TrogdorError::Image(e) => write!(f, "Image Error: {}", e),
            TrogdorError::Serial(e) => write!(f, "Serial Error: {}", e),
            TrogdorError::Env(e) => write!(f, "Environment Variable Error: {}", e),
            TrogdorError::PicoArgs(e) => write!(f, "CLI Argument Error: {}", e),
            TrogdorError::Usvg(e) => write!(f, "SVG Error: {}", e),
            TrogdorError::Generic(s) => write!(f, "Error: {}", s),
        }
    }
}

impl Error for TrogdorError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            TrogdorError::Io(e) => Some(e),
            TrogdorError::ParseFloat(e) => Some(e),
            TrogdorError::Json(e) => Some(e),
            TrogdorError::Image(e) => Some(e),
            TrogdorError::Serial(e) => Some(e),
            TrogdorError::Env(e) => Some(e),
            TrogdorError::PicoArgs(e) => Some(e),
            TrogdorError::Usvg(e) => Some(e),
            TrogdorError::Generic(_) => None,
        }
    }
}

impl From<std::io::Error> for TrogdorError {
    fn from(e: std::io::Error) -> Self {
        TrogdorError::Io(e)
    }
}

impl From<std::num::ParseFloatError> for TrogdorError {
    fn from(e: std::num::ParseFloatError) -> Self {
        TrogdorError::ParseFloat(e)
    }
}

impl From<serde_json::Error> for TrogdorError {
    fn from(e: serde_json::Error) -> Self {
        TrogdorError::Json(e)
    }
}

impl From<image::ImageError> for TrogdorError {
    fn from(e: image::ImageError) -> Self {
        TrogdorError::Image(e)
    }
}

impl From<serialport::Error> for TrogdorError {
    fn from(e: serialport::Error) -> Self {
        TrogdorError::Serial(e)
    }
}

impl From<std::env::VarError> for TrogdorError {
    fn from(e: std::env::VarError) -> Self {
        TrogdorError::Env(e)
    }
}

impl From<pico_args::Error> for TrogdorError {
    fn from(e: pico_args::Error) -> Self {
        TrogdorError::PicoArgs(e)
    }
}

impl From<usvg::Error> for TrogdorError {
    fn from(e: usvg::Error) -> Self {
        TrogdorError::Usvg(e)
    }
}

impl From<String> for TrogdorError {
    fn from(s: String) -> Self {
        TrogdorError::Generic(s)
    }
}

impl From<&str> for TrogdorError {
    fn from(s: &str) -> Self {
        TrogdorError::Generic(s.to_string())
    }
}

impl From<Box<dyn Error + Send + Sync>> for TrogdorError {
    fn from(e: Box<dyn Error + Send + Sync>) -> Self {
        TrogdorError::Generic(e.to_string())
    }
}

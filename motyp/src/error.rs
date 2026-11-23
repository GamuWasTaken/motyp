#[derive(Debug)]
pub enum Error {
    Config(String),
    Path(String),
    Io(std::io::Error),
    Typst(String),
    Render(String),
    Yuv(yuv::YuvError), // TODO Check what can produce errors in Yuv, Muxer and openh264, we really shouldnt need them as long as we dont feed them garbo
    Encode(openh264::Error),
    Mux(muxide::api::MuxerError),
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<yuv::YuvError> for Error {
    fn from(value: yuv::YuvError) -> Self {
        Self::Yuv(value)
    }
}
impl From<openh264::Error> for Error {
    fn from(value: openh264::Error) -> Self {
        Self::Encode(value)
    }
}
impl From<muxide::api::MuxerError> for Error {
    fn from(value: muxide::api::MuxerError) -> Self {
        Self::Mux(value)
    }
}
impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Config(message)
            | Error::Path(message)
            | Error::Typst(message)
            | Error::Render(message) => write!(f, "{message}"),

            Error::Io(message) => write!(f, "Io error: {message}"),
            Error::Yuv(message) => write!(f, "Yuv error: {message}"),
            Error::Encode(message) => write!(f, "Encode error: {message}"),
            Error::Mux(message) => write!(f, "Mux error: {message}"),
        }
    }
}

impl core::error::Error for Error {}

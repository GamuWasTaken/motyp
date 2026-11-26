use std::path::PathBuf;

use typst::diag::SourceDiagnostic;

#[derive(Debug)]
pub enum Warning {
    // Input file generated more than 1 page
    IgnoredPages(u32),
    // Typst warnings
    Typst(SourceDiagnostic),
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Warning::IgnoredPages(frame) => write!(
                f,
                "Frame #{frame} generated more than 1 page, only the first one is used"
            ),
            Warning::Typst(source_diagnostic) => {
                // MAYBE: print typst hints too
                write!(f, "Typst compiler emitted: {}", source_diagnostic.message)
            }
        }
    }
}

#[derive(Debug)]
pub enum Error {
    // Cant parse input file
    // Cant find input file
    // Cant read input file
    // Cant create output file
    // Cant write to output file
    IO(String),

    // Cant initialize openh264 encoder (cannot happen)
    // Cant initialize muxer (size is out of 0..u16::MAX)
    // Cant convert rgba to yuv420 (size changed from init to compile, cannot happen)
    Config(String),

    // Cant compile input file
    // Input file generates no pages TODO: warn if more than one is generated
    // Generated frame does not match config
    Typst(String),

    // Muxer errored out
    // OpenH264 errored out
    Encoding(String),
}
impl Error {
    /// Cant parse input file
    pub fn bad_input_path(path: &PathBuf) -> Self {
        Self::IO(format!("can not parse input path: '{:#?}'", path))
    }
    /// Cant find input file
    pub fn input_not_found(error: typst::syntax::PathError) -> Self {
        Self::IO(format!("can not find input file: '{error}'"))
    }
    /// Cant read input file
    pub fn input_not_readable(error: typst::diag::FileError) -> Self {
        Self::IO(format!("can not read input file: '{error}'"))
    }
    /// Cant create output file
    pub fn output_not_creatable(error: std::io::Error) -> Self {
        Self::IO(format!("can not create output file: '{error}'"))
    }
    /// Cant write to output file
    pub fn output_not_writeable(error: muxide::api::MuxerError) -> Self {
        Self::IO(format!("can not write to output file: '{error}'"))
    }
    /// Cant initialize openh264 encoder (cannot happen)
    pub fn openh264_init(error: openh264::Error) -> Self {
        Self::Config(format!("can not initialize OpenH264: '{error}'"))
    }
    /// Cant initialize muxer (size is out of 0..u16::MAX)
    pub fn muxer_init(error: muxide::api::MuxerError) -> Self {
        Self::Config(format!("can not initialize Muxer: '{error}'"))
    }
    /// Cant convert rgba to yuv420 (size changed from init to compile, cannot happen)
    pub fn yuv_failed(error: yuv::YuvError) -> Self {
        Self::Encoding(format!("can not convert rgba to yuv420: '{error}'"))
    }
    /// Muxer errored out
    pub fn muxer_failed(error: muxide::api::MuxerError) -> Self {
        Self::Encoding(format!("muxer failed: '{error}'"))
    }
    /// OpenH264 errored out
    pub fn openh264_failed(error: openh264::Error) -> Self {
        Self::Encoding(format!("OpenH264 failed: '{error}'"))
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IO(error)
            | Error::Config(error)
            | Error::Typst(error)
            | Error::Encoding(error) => f.write_str(error),
        }
    }
}

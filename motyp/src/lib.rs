use core::fmt;
use std::path::PathBuf;

use crate::{
    encoder::Encoder,
    renderer::{Renderer, has_end_marker},
};
// TODO maybe do more than tune the encoder with this info (dev could be watch, and release standalone call, this makes it simpler to use (less options same functionality))
#[derive(Debug, Clone)]
pub enum Profile {
    Dev,
    Release,
}
#[derive(Debug, Clone)]
pub struct Config {
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub output: PathBuf,
    pub input: PathBuf,
    pub bitrate: u32,
    pub profile: Profile,
    pub data: Vec<(String, String)>,
}

pub struct Tool {
    renderer: Renderer,
    encoder: Encoder,
}

impl Tool {
    pub fn new(config: Config) -> Result<Self, Error> {
        Ok(Self {
            renderer: Renderer::new(config.clone())?,
            encoder: Encoder::new(config.clone())?,
        })
    }

    pub fn generate_video(mut self) -> Result<(), Error> {
        loop {
            let compiled = self.renderer.compile_frame()?;
            let end = has_end_marker(&compiled.document);
            let rendered = self.renderer.render_frame(compiled)?;

            self.encoder.push_frame(&rendered)?;
            self.renderer.frame += 1;

            if end {
                break;
            }
        }
        self.encoder.finish()?;
        Ok(())
    }
}

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
impl fmt::Display for Error {
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

mod encoder {
    use std::fs::File;

    use muxide::api::{MuxerBuilder, VideoCodec};
    use openh264::{
        OpenH264API,
        encoder::{BitRate, Complexity, EncoderConfig, FrameRate, FrameType, RateControlMode},
        formats::YUVSlices,
    };
    use yuv::{YuvConversionMode, YuvRange, YuvStandardMatrix, rgba_to_yuv420};

    use crate::{Config, Error, renderer::RenderedFrame};

    pub struct Encoder {
        config: Config,
        encoder: openh264::encoder::Encoder,
        muxer: muxide::api::Muxer<File>,
        yuv: yuv::YuvPlanarImageMut<'static, u8>,
        bitstream: Vec<u8>,
    }

    impl Encoder {
        pub fn new(config: Config) -> Result<Self, Error> {
            let rate_control_mode = match config.profile {
                crate::Profile::Dev => RateControlMode::Bufferbased,
                crate::Profile::Release => RateControlMode::Off,
            };

            let encoder_config = EncoderConfig::new()
                .bitrate(BitRate::from_bps(config.bitrate))
                .max_frame_rate(FrameRate::from_hz(config.fps))
                .rate_control_mode(rate_control_mode)
                .skip_frames(false)
                .adaptive_quantization(false);

            let encoder_config = match config.profile {
                crate::Profile::Dev => encoder_config
                    .complexity(Complexity::Low)
                    .background_detection(false)
                    .scene_change_detect(false),
                crate::Profile::Release => encoder_config
                    .complexity(Complexity::High)
                    .background_detection(true)
                    .scene_change_detect(true),
            };

            let encoder = openh264::encoder::Encoder::with_api_config(
                OpenH264API::from_source(),
                encoder_config,
            )?;

            let file = File::create(&config.output)?;
            let muxer = MuxerBuilder::new(file)
                .video(
                    VideoCodec::H264,
                    config.width,
                    config.height,
                    config.fps as _,
                )
                .build()?;
            let yuv = yuv::YuvPlanarImageMut::alloc(
                config.width,
                config.height,
                yuv::YuvChromaSubsampling::Yuv420,
            );

            Ok(Self {
                config,
                encoder,
                muxer,
                yuv,
                bitstream: Vec::new(),
            })
        }

        pub fn push_frame(&mut self, frame: &RenderedFrame) -> Result<(), Error> {
            rgba_to_yuv420(
                &mut self.yuv,
                &frame.data,
                self.config.width * 4,
                YuvRange::Limited,
                YuvStandardMatrix::Bt601,
                YuvConversionMode::Balanced,
            )?;

            let source = YUVSlices::new(
                (
                    self.yuv.y_plane.borrow(),
                    self.yuv.u_plane.borrow(),
                    self.yuv.v_plane.borrow(),
                ),
                (self.config.width as usize, self.config.height as usize),
                (
                    self.yuv.y_stride as usize,
                    self.yuv.u_stride as usize,
                    self.yuv.v_stride as usize,
                ),
            );

            let encoded_stream = self.encoder.encode(&source)?;
            self.bitstream.clear();
            encoded_stream.write_vec(&mut self.bitstream);

            self.muxer.write_video(
                (frame.id as f32 / self.config.fps) as _,
                &self.bitstream,
                matches!(encoded_stream.frame_type(), FrameType::IDR | FrameType::I),
            )?;

            Ok(())
        }

        pub fn finish(self) -> Result<(), Error> {
            self.muxer.finish().map_err(Into::into)
        }
    }
}

mod renderer {
    use std::{ffi::OsStr, path::Path};

    use super::{Config, Error};

    use tiny_skia::Pixmap;
    use typst::{
        Library, LibraryExt, World,
        diag::{FileResult, SourceDiagnostic},
        foundations::{Bytes, Datetime, Dict, Duration, NativeElement, Value},
        introspection::{Introspector, MetadataElem},
        syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot},
        text::{Font, FontBook},
        utils::{LazyHash, Scalar},
    };
    use typst_kit::{
        datetime::Time,
        downloader::SystemDownloader,
        files::{FileStore, FsRoot, SystemFiles},
        fonts::{FontSource, FontStore},
        packages::SystemPackages,
    };
    use typst_layout::PagedDocument;

    // TODO
    // -[x] Compile
    // -[ ] Watch

    pub struct Renderer {
        pub frame: u32,
        config: Config,
        library: LazyHash<Library>,
        fonts: FontStore,
        files: FileStore<SystemFiles>,
        main: FileId,
        time: Time,
    }

    impl Renderer {
        pub fn new(config: Config) -> Result<Self, Error> {
            let root = config
                .input
                .parent()
                .filter(|p| !p.is_empty())
                .unwrap_or(Path::new("."))
                .to_path_buf();

            let file_name = config
                .input
                .file_name()
                .and_then(OsStr::to_str)
                .ok_or(Error::Path(
                    "source path has no valid UTF-8 file name".into(),
                ))?;

            let main = RootedPath::new(
                VirtualRoot::Project,
                VirtualPath::new(file_name)
                    .map_err(|e| Error::Path(format!("invalid source path: {e}")))?,
            )
            .intern();

            let files = FileStore::new(SystemFiles::new(
                FsRoot::new(root),
                SystemPackages::new(SystemDownloader::new("motyp")),
            ));
            files.source(main).map_err(|e| {
                Error::Path(format!("could not read {}: {e}", config.input.display()))
            })?;

            let library = LazyHash::new(
                Library::builder()
                    .with_inputs(Dict::from_iter(config.data.iter().map(|(key, val)| {
                        (key.as_str().into(), Value::Str(val.as_str().into()))
                    })))
                    .build(),
            );

            let time = Time::fixed_timestamp(0).map_err(|e| Error::Typst(e.to_string()))?;

            let mut fonts = FontStore::new();
            fonts.extend(
                typst_kit::fonts::system()
                    .filter_map(|(source, info)| source.load().map(|font| (font, info))),
            );
            fonts.extend(typst_kit::fonts::embedded());

            Ok(Self {
                config,
                library,
                fonts,
                files,
                main,
                time,
                frame: 0,
            })
        }

        pub fn compile_frame(&self) -> Result<CompiledFrame, Error> {
            let warned = typst::compile::<PagedDocument>(self);
            let document = warned
                .output
                .map_err(|e| Error::Typst(format_typst_errors(&e)))?; // MAYBE: we can return the typst diagnostic and handle it elsewhere

            let warnings = warned
                .warnings
                .iter()
                .map(|w| w.message.to_string())
                .collect();

            Ok(CompiledFrame { document, warnings })
        }

        pub fn render_frame(&self, compiled: CompiledFrame) -> Result<RenderedFrame, Error> {
            let page = compiled
                .document
                .pages()
                .first()
                .ok_or(Error::Typst("document has no pages".into()))?;
            let pixmap = typst_render::render(
                &page,
                &typst_render::RenderOptions {
                    pixel_per_pt: Scalar::new(
                        f64::from(self.config.width) / page.frame.size().x.to_pt(),
                    ),
                    render_bleed: false,
                },
            );

            self.validate_frame(&pixmap)?;

            Ok(RenderedFrame {
                id: self.frame,
                data: pixmap.take(),
            })
        }

        fn validate_frame(&self, data: &Pixmap) -> Result<(), Error> {
            let (width, height) = (data.width(), data.height());

            if width != self.config.width || height != self.config.height {
                return Err(Error::Config(format!(
                    "frame is {}x{}, expected {}x{}",
                    width, height, self.config.width, self.config.height
                )));
            }

            Ok(())
        }
    }

    fn format_typst_errors(errors: &[SourceDiagnostic]) -> String {
        errors
            .iter()
            .map(|e| {
                let mut message = e.message.to_string();
                for hint in &e.hints {
                    message.push_str("\nhint: ");
                    message.push_str(&hint.v);
                }
                message
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub const END_MARKER: &str = "motyp_end";
    pub fn has_end_marker(page: &PagedDocument) -> bool {
        page.introspector()
            .query(&MetadataElem::ELEM.select())
            .iter()
            .any(|content| match content.get_by_name("value") {
                Ok(Value::Str(text)) => text.as_str() == END_MARKER,
                _ => false,
            })
    }

    pub struct CompiledFrame {
        pub document: PagedDocument,
        pub warnings: Vec<String>,
    }

    pub struct RenderedFrame {
        pub data: Vec<u8>,
        pub id: u32,
    }

    impl World for Renderer {
        fn library(&self) -> &LazyHash<Library> {
            &self.library
        }

        fn book(&self) -> &LazyHash<FontBook> {
            self.fonts.book()
        }

        fn main(&self) -> FileId {
            self.main
        }

        fn source(&self, id: FileId) -> FileResult<Source> {
            if is_frame_file(id) {
                Ok(Source::new(id, self.frame.to_string()))
            } else {
                self.files.source(id)
            }
        }

        fn file(&self, id: FileId) -> FileResult<Bytes> {
            if is_frame_file(id) {
                Ok(Bytes::from_string(self.frame.to_string()))
            } else {
                self.files.file(id)
            }
        }

        fn font(&self, index: usize) -> Option<Font> {
            self.fonts.font(index)
        }

        fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
            self.time.today(offset)
        }
    }

    fn is_frame_file(id: FileId) -> bool {
        matches!(id.root(), VirtualRoot::Project) && id.vpath().file_name() == Some("motyp_frame")
    }
}

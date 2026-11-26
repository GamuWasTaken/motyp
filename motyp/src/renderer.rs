use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use crate::error::Warning;

use super::Error;

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

pub struct Config {
    pub data: Vec<(String, String)>,
    pub height: u32,
    pub input: PathBuf,
    pub width: u32,
}

impl From<&super::Config> for Config {
    fn from(value: &super::Config) -> Self {
        Self {
            width: value.width,
            height: value.height,
            input: value.input.clone(),
            data: value.inputs.clone(),
        }
    }
}

impl TryFrom<&super::Config> for Renderer {
    type Error = Error;

    fn try_from(value: &super::Config) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

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
    pub fn new(config: impl Into<Config>) -> Result<Self, Error> {
        let config = config.into();
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
            .ok_or(Error::bad_input_path(&config.input))?;

        let main = RootedPath::new(
            VirtualRoot::Project,
            VirtualPath::new(file_name).map_err(Error::input_not_found)?,
        )
        .intern();

        let files = FileStore::new(SystemFiles::new(
            FsRoot::new(root),
            SystemPackages::new(SystemDownloader::new("motyp")),
        ));
        files.source(main).map_err(Error::input_not_readable)?;

        let library = LazyHash::new(
            Library::builder()
                .with_inputs(Dict::from_iter(config.data.iter().map(|(key, val)| {
                    (key.as_str().into(), Value::Str(val.as_str().into()))
                })))
                .build(),
        );

        let time = Time::system();

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
            .map_err(|e| Error::Typst(format_typst_errors(&e)))?;

        let mut warnings: Vec<_> = warned.warnings.into_iter().map(Warning::Typst).collect();

        if document.pages().len() != 1 {
            warnings.push(Warning::IgnoredPages(self.frame));
        }

        Ok(CompiledFrame { document, warnings })
    }

    pub fn render_frame(&self, compiled: PagedDocument) -> Result<RenderedFrame, Error> {
        let page = compiled
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

        Ok(RenderedFrame {
            id: self.frame,
            data: pixmap,
        })
    }

    pub fn validate_render(&self, frame: &RenderedFrame) -> Result<(), Error> {
        let (width, height) = (frame.data.width(), frame.data.height());

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

pub const CONTINUE_MARKER: &str = "motyp_continue";
pub fn has_continue_marker(page: &PagedDocument) -> bool {
    page.introspector()
        .query(&MetadataElem::ELEM.select())
        .iter()
        .any(|content| match content.get_by_name("value") {
            Ok(Value::Str(text)) => text.as_str() == CONTINUE_MARKER,
            _ => false,
        })
}

pub struct CompiledFrame {
    pub document: PagedDocument,
    pub warnings: Vec<Warning>,
}

pub struct RenderedFrame {
    pub data: Pixmap,
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

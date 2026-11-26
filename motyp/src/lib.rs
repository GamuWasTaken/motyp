use std::path::PathBuf;

mod encoder;
mod error;
mod renderer;

pub use crate::{encoder::Encoder, error::Error, renderer::Renderer};
use crate::{
    error::Warning,
    renderer::{CompiledFrame, has_continue_marker},
};

// TODO maybe do more than tune the encoder with this info (dev could be watch, and release standalone call, this makes it simpler to use (less options same functionality))
#[derive(Debug, Clone, Copy)]
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
    pub inputs: Vec<(String, String)>,
}

pub struct Tool;

pub struct Stats {
    pub frames: u32,
    pub warnings: Vec<Warning>,
}

impl Tool {
    pub fn generate_video(config: impl Into<Config>) -> Result<Stats, Error> {
        let config = &config.into();
        let mut renderer: Renderer = config.try_into()?;
        let mut encoder: Encoder = config.try_into()?;
        let mut warning_collector = Vec::new();
        loop {
            let CompiledFrame { document, warnings } = renderer.compile_frame()?;
            if !has_continue_marker(&document) {
                break;
            }

            warning_collector.extend(warnings);

            let rendered = renderer.render_frame(document)?;

            encoder.push_frame(rendered)?;
            renderer.frame += 1;
        }
        encoder.finish()?;
        Ok(Stats {
            frames: renderer.frame,
            warnings: warning_collector,
        })
    }

    pub fn validate_video(config: impl Into<Config>) -> Result<Stats, Error> {
        let config = &config.into();
        let mut renderer: Renderer = config.try_into()?;
        let mut warning_collector = Vec::new();
        loop {
            let CompiledFrame { document, warnings } = renderer.compile_frame()?;
            if !has_continue_marker(&document) {
                break;
            }

            warning_collector.extend(warnings);

            let rendered = renderer.render_frame(document)?;
            renderer.validate_render(&rendered)?;
            renderer.frame += 1;
        }

        Ok(Stats {
            frames: renderer.frame,
            warnings: warning_collector,
        })
    }
}

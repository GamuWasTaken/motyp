use std::path::PathBuf;

mod encoder;
mod error;
mod renderer;

use crate::{
    encoder::Encoder,
    error::Error,
    renderer::{Renderer, has_continue_marker},
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

pub struct Tool {
    renderer: Renderer,
    encoder: Encoder,
}

impl Tool {
    pub fn new(config: &Config) -> Result<Self, Error> {
        Ok(Self {
            renderer: Renderer::new(config)?,
            encoder: Encoder::new(config)?,
        })
    }

    // TODO return video stats
    pub fn generate_video(mut self) -> Result<(), Error> {
        loop {
            let compiled = self.renderer.compile_frame()?;
            if !has_continue_marker(&compiled.document) {
                break;
            }

            let rendered = self.renderer.render_frame(compiled)?;

            self.encoder.push_frame(&rendered)?;
            self.renderer.frame += 1;
        }
        self.encoder.finish()?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), Error> {
        todo!()
    }
}

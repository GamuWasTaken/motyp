use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, ValueEnum};

#[derive(Parser)]
#[command(version, about, color=clap::ColorChoice::Always)]
pub struct Cli {
    #[arg(value_enum)]
    pub command: Commands,

    #[command(flatten)]
    pub config: Config,
}
impl From<&Config> for motyp::Config {
    fn from(value: &Config) -> Self {
        Self {
            width: value.width as _,
            height: value.height as _,
            fps: value.fps as _,
            output: value.output.clone(),
            input: value.input.clone(),
            bitrate: value.bitrate,
            profile: motyp::Profile::Dev,
            inputs: value.inputs.clone(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum Commands {
    Compile,
    Validate,
}

#[derive(Args)]
pub struct Config {
    #[clap(long, default_value_t = 1920)]
    pub width: u16,

    #[clap(long, default_value_t = 1080)]
    pub height: u16,

    #[clap(long, default_value_t = 60)]
    pub fps: u8,

    #[clap(short, long, default_value = "out.mp4")]
    pub output: PathBuf,

    pub input: PathBuf,

    #[clap(long, default_value_t = 10_000_000)]
    pub bitrate: u32,

    #[clap(
        long,
        value_name = "key=value",
        action = ArgAction::Append,
        value_parser = parse_input_pair
    )]
    pub inputs: Vec<(String, String)>,
}

fn parse_input_pair(raw: &str) -> Result<(String, String), String> {
    let (key, val) = raw
        .split_once("=")
        .ok_or("input must be a key and a value separated by an equal sign")?;
    let key = key.trim().to_owned();
    if key.is_empty() {
        return Err("the key was missing or empty".into());
    }
    let val = val.trim().to_owned();
    Ok((key, val))
}

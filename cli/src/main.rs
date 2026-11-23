use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand};
use motyp::Tool;
#[derive(Parser)]
#[command(version, about, color=clap::ColorChoice::Always)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
impl TryFrom<Cli> for motyp::Config {
    type Error = Result<Self, String>;

    fn try_from(value: Cli) -> Result<Self, Self::Error> {
        match value.command {
            Commands::Compile(config) => Ok(Self {
                width: config.width,
                height: config.height,
                fps: config.fps,
                output: config.output,
                input: config.input,
                bitrate: config.bitrate,
                profile: motyp::Profile::Dev,
                inputs: config.inputs,
            }),
            _ => todo!(),
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Compile a video from the given .typ file")]
    Compile(Config),
    #[command(about = "Watch the given .typ file and recompile on change")]
    Watch(Config),
    #[command(about = "Check the given .typ file is correct and print stats")]
    Validate(Config),
}

#[derive(Args)]
struct Config {
    #[clap(short, long, default_value_t = 1920)]
    width: u32,

    #[clap(short, long, default_value_t = 1080)]
    height: u32,

    #[clap(long, default_value_t = 60.)]
    fps: f32,

    #[clap(short, long, default_value = "out.mp4")]
    output: PathBuf,

    input: PathBuf,

    #[clap(long, default_value_t = 10_000_000)]
    bitrate: u32,

    #[clap(
        long,
        value_name = "key=value",
        action = ArgAction::Append,
        value_parser = parse_input_pair
    )]
    inputs: Vec<(String, String)>,
}

fn main() {
    let config = Cli::parse().try_into().unwrap();

    let tool = match Tool::new(&config) {
        Ok(tool) => tool,
        Err(message) => {
            println!("{message}");
            return;
        }
    };

    let res = tool.generate_video();

    let _ = dbg!(res, config);
}

/*

motyp compile|watch|validate

compile: generates a video from input
watch: compile on input or dependencies change
validate: check video will generate correctly and print stats (width, height, fps, frame ammount, video length)

TODO make a progress bar (only when we know the frame amount/we can have it as an option, if asked, run the loop just compiling (validate), and then rendering with the progress bar)

*/
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

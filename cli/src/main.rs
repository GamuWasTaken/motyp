use clap::Parser;
use motyp::Tool;

mod args;
use crate::args::Cli;

fn main() {
    let args = Cli::parse();

    let config = args.config;
    let action = match args.command {
        args::Commands::Compile => Tool::generate_video,
        args::Commands::Validate => Tool::validate_video,
    };

    let res = action(&config);

    // TODO improve warning and error reporting
    match res {
        Ok(stats) => {
            println!(
                "Done! Generated {} seconds",
                stats.frames as f32 / config.fps as f32
            );

            if !stats.warnings.is_empty() {
                println!("With the following warnings:");
                for warning in &stats.warnings {
                    println!("\t{warning}")
                }
            }
        }
        Err(error) => println!("Ups! {error}"),
    }
}

/*

motyp compile|validate

compile: generates a video from input
validate: check video will generate correctly and print stats (width, height, fps, frame ammount, video length)

TODO make a progress bar (only when we know the frame amount/we can have it as an option, if asked, run the loop just compiling (validate), and then rendering with the progress bar)
TODO: create temp then rename to avoid incomplete overwrites

DISCUSSION: Do we really need watch? We dont really benefit much from having watch in the tool (can be done externally without loss of performance)

*/

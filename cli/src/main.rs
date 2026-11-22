use motyp::{Config, Tool};
fn main() {
    let config = Config {
        width: 1920,
        height: 1080,
        fps: 60.,
        output: "out.mp4".into(),
        input: "in.typ".into(),
        bitrate: 2_000_000,
        profile: motyp::Profile::Dev,
        data: vec![],
    };

    let tool = match Tool::new(config) {
        Ok(tool) => tool,
        Err(message) => {
            println!("{message}");
            return;
        }
    };

    let res = tool.generate_video();

    dbg!(res);
}

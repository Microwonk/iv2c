use clap::{Parser, ValueEnum};
use iv2c::error::Error;
use iv2c::frames::{MediaData, open_media_from_path};
use iv2c::maps::CharMap;
use iv2c::pipeline::{ImagePipeline, Resolution};
use iv2c::render::{RenderFrame, RenderOptions};

mod terminal_player;

/// Command line arguments structure.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Play or Export
    #[arg(value_enum, required = true)]
    action: Action,
    /// Name of the file/stream to process
    #[arg(required = true)]
    input: String,
    // Name of the file to output to
    #[arg(short, long)]
    output: Option<String>,
    /// Force a user-specified FPS
    #[arg(short, long)]
    fps: Option<String>,
    /// Loop playing of video/gif
    #[arg(short, long, default_value_t = false)]
    r#loop: bool,
    /// Custom lookup char table
    #[arg(short, long)]
    char_map: Option<String>,
    /// Grayscale mode
    #[arg(short, long, default_value_t = false)]
    gray: bool,
    /// Experimental width modifier (emojis have 2x width)
    #[arg(short, long, default_value_t = 1)]
    w_mod: u32,
    /// Experimental frame skip flag
    #[arg(short, long, default_value_t = false)]
    allow_frame_skip: bool,
    /// Experimental flag to add newlines
    #[arg(short, long, default_value_t = false)]
    new_lines: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
#[clap(rename_all = "lower")]
enum Action {
    Export,
    Play,
}

const DEFAULT_FPS: f64 = 30.0;

use std::path::Path;

use crate::terminal_player::TerminalPlayer;

fn main() -> Result<(), Error> {
    let args = Args::parse();

    let path = args.input.clone();

    let media_data = open_media_from_path(Path::new(&path))?;

    match args.action {
        Action::Export => export(args, media_data),
        Action::Play => play(args, media_data),
    }
}

fn export(_args: Args, _media_data: MediaData) -> Result<(), Error> {
    Ok(())
}

fn play(args: Args, media_data: MediaData) -> Result<(), Error> {
    let media = media_data.frame_iter;
    let fps = media_data.fps;

    let mut term = TerminalPlayer::new("Title".to_string(), args.gray);

    term.init()?;

    let (width, height) = TerminalPlayer::size().map(|(w, h)| (w as u32, h as u32))?;

    let mut use_fps = DEFAULT_FPS;
    if let Some(fps) = fps {
        use_fps = fps;
    }
    if let Some(fps) = &args.fps {
        use_fps = fps
            .parse::<f64>()
            .map_err(|err| Error::Application(format!("Data error: {err:?}")))?;
    }
    let cmaps = args
        .char_map
        .clone()
        .map_or(CharMap::Dotted, |s| CharMap::custom(&s));
    let w_mod = args.w_mod;
    let allow_frame_skip = args.allow_frame_skip;
    let new_lines = args.new_lines;
    let loop_playback = args.r#loop;

    let mut renderer = iv2c::render::Renderer::new(
        ImagePipeline::new(Resolution::Fixed(width, height), cmaps, new_lines),
        media,
        RenderOptions {
            fps: use_fps,
            w_mod,
            loop_playback,
        },
    );

    renderer.run(allow_frame_skip, term.callback())?;
    Ok(())
}

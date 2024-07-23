use anyhow::Result;
use clap::Parser;
use crate::args::{Args, Mode};

mod args;
mod server;

fn main() -> Result<()> {
    let args = Args::parse();

    match args.mode {
        Mode::Server => { server::start(args.port)?; }
        Mode::Client => { unimplemented!("Client isn't available yet"); }
    }

    Ok(())
}

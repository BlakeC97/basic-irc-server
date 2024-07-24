use std::io::{stdin, stdout};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use anyhow::Result;
use clap::Parser;
use crate::args::{Args, Mode};
use crate::user::User;

mod args;
mod server;
mod client;
mod user;
mod server_friendly_string;

fn main() -> Result<()> {
    let args = Args::parse();

    match args.mode {
        Mode::Server => { server::start(args.port)?; }
        Mode::Client => {
            let name = args.name.unwrap_or_else(|| {
                client::get_input(b"Enter a username: ", stdin().lock(), stdout().lock())
                    .expect("Couldn't get username")
            });
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), args.port);

            client::start(User::new(name), addr)?;
        }
    }

    Ok(())
}

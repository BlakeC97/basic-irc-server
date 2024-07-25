use std::io::{stdin, stdout};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use anyhow::Result;
use clap::Parser;
use crate::args::{Args, Mode};
use crate::user::User;
use crate::server::Server;
use crate::client::Client;

mod args;
mod server;
mod client;
mod user;
mod server_friendly_string;
mod response;

fn main() -> Result<()> {
    let args = Args::parse();

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), args.port);
    match args.mode {
        Mode::Server => {
            let listener = TcpListener::bind(addr)?;
            eprintln!("Listening on port {}", listener.local_addr().expect("Can't get local_addr for server").port());
            Server::new(listener).start()?;
        }
        Mode::Client => {
            let name = args.name.unwrap_or_else(|| {
                client::get_input(b"Enter a username: ", stdin().lock(), stdout().lock())
                    .expect("Couldn't get username")
            });

            Client::new(User::new(name), TcpStream::connect(addr)?).start()?;
        }
    }

    Ok(())
}

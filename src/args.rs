use clap::Parser;
use thiserror::Error;

#[derive(clap::ValueEnum, Debug, Clone)]
pub enum Mode {
    Client,
    Server,
}

#[derive(Error, Debug)]
pub enum ArgError {
    #[error("Invalid input: `{0}`")]
    InvalidInput(String),
    #[error("No input given -- please pass 'client' or 'server'")]
    NoInput,
}

impl TryFrom<String> for Mode {
    type Error = ArgError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value == "client" {
            Ok(Mode::Client)
        } else if value == "server" {
            Ok(Mode::Server)
        } else if value.is_empty() {
            Err(ArgError::NoInput)
        } else {
            Err(ArgError::InvalidInput(value.clone()))
        }
    }
}

#[derive(Parser, Debug)]
#[command(about, about = "Does a TCP server/client thing.")]
pub struct Args {
    #[arg(short, long, help = "Mode to start the app in.")]
    pub mode: Mode,
    #[arg(short, long, help = "Port to use. Default will bind any available port", default_value_t = 0)]
    pub port: u16,
}
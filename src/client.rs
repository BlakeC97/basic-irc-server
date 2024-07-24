use std::io::{BufRead, stdin, stdout, Write};
use std::net::{SocketAddr, TcpStream};
use thiserror::Error;
use crate::server_friendly_string::ServerFriendlyString;
use crate::user::User;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Failed to read/write from stream: `{0}`")]
    IO(#[from] std::io::Error),
    #[error("Failed serializing user info: `{0}`")]
    Serde(#[from] serde_json::Error),
}

/// Reads input using a given prompt up to the first newline.
pub fn get_input<I, O>(prompt: &[u8], mut input: I, mut output: O) -> Result<String, std::io::Error>
where
    I: BufRead,
    O: Write,
{
    output.write_all(prompt)?;
    output.flush()?;

    let mut read = String::with_capacity(64);
    input.read_line(&mut read)?;

    Ok(read)
}

pub fn start(user: User, address: SocketAddr) -> Result<(), ClientError> {
    let mut client = TcpStream::connect(address)?;

    let user_str = serde_json::to_vec(&user)?;
    client.write_all(&user_str)?;

    loop {
        let msg = match get_input(b"> ", stdin().lock(), stdout().lock()) {
            Ok(m) => {
                if m.is_empty() {
                    break;
                }

                ServerFriendlyString::from(m)
            }
            Err(e) => {
                eprintln!("Couldn't get input, skipping: {e:?}");
                continue;
            }
        };

        if let Err(e) = client.write_all(msg.0.as_bytes()) {
            eprintln!("Couldn't write message; skipping: {e:?}");
            continue;
        }

        println!("<{}> {}", user.name, msg);
    }

    Ok(())
}


#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use super::*;

    #[test]
    fn test_get_input() {
        let input_str = "what the dog doin\nthis won't be read";
        let mut input = Cursor::new(input_str);
        let mut output = Vec::with_capacity(128);

        let res = get_input(b"Snart: ", &mut input, &mut output).unwrap();
        assert_eq!("what the dog doin\n", res);
        assert_eq!(input_str.chars().position(|c| c == '\n').unwrap(), (input.position() - 1) as usize);
        assert_eq!("this won't be read", &input.get_ref()[input.position() as usize..]);
        assert_eq!(b"Snart: ", &output[..]);
    }

    #[test]
    fn test_get_input_no_newline() {
        let input_str = "this is just one long line no newline";
        let mut input = Cursor::new(input_str);
        let mut output = Vec::with_capacity(128);

        let res = get_input(b"> ", &mut input, &mut output).unwrap();
        assert_eq!("this is just one long line no newline", res);
        assert_eq!(input_str.len(), input.position() as usize);
        assert_eq!("", &input.get_ref()[input.position() as usize..]);
        assert_eq!(b"> ", &output[..]);
    }
}
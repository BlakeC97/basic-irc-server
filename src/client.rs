use std::io::{BufRead, Read, stdin, stdout, Write};
use std::sync::Arc;
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use thiserror::Error;
use parking_lot::Mutex;
use crate::response::AuthResponse;
use crate::scuffed_clone::ScuffedClone;
use crate::server::VALIDATE_BUFFER_SIZE;
use crate::server_friendly_string::ServerFriendlyString;
use crate::user::User;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Failed to read/write from stream: `{0}`")]
    IO(#[from] std::io::Error),
    #[error("Failed serializing user info: `{0}`")]
    Serde(#[from] serde_json::Error),
    #[error("Authorization failed: `{0}`")]
    Auth(#[from] AuthResponse),
}

#[derive(Debug)]
pub struct Client<S: Read + Write + ScuffedClone + Send> {
    user: User,
    conn: S,
}

impl<S: Read + Write + ScuffedClone + Send> Client<S>
{
    pub fn new(user: User, conn: S) -> Self {
        Self {
            user,
            conn,
        }
    }

    /// Performs the authorization flow for a connecting user. In addition to the `Result`, this function
    /// reads an `AuthResponse` from the server indicating success or failure.
    fn do_auth_flow(&mut self) -> Result<(), ClientError> {
        let user_str = serde_json::to_vec(&self.user)?;
        self.conn.write_all(&user_str)?;

        let mut buf = [0; VALIDATE_BUFFER_SIZE * 2];
        let n = self.conn.read(&mut buf)?;
        // Don't read the null bytes
        let resp: AuthResponse = serde_json::from_slice(&buf[..n])?;

        match &resp {
            AuthResponse::Success => Ok(()),
            AuthResponse::Error(_) => Err(ClientError::Auth(resp)),
        }
    }

    pub fn start(&mut self) -> Result<(), ClientError> {
        self.do_auth_flow()?;

        // Concurrency is hard so I'll do it stupidly. Yes that's a Mutex for a stream that will
        // _always_ exclusively hold it. I'm stupid.
        // TODO: This DOES NOT work as expected. A thread scope will block waiting for the threads to
        //       return, so it's not suitable for what I'm trying to do. I'll need to be able to use
        //       a regular `thread::spawn` call, which requires a 'static lifetime on `S`. God help me.
        //let arc_conn = Arc::new(Mutex::new(self.conn.scuffed_clone()));
        //thread::scope(|scope| {
        //    let rx_conn = arc_conn.clone();
        //    scope.spawn(move || {
        //        let mut rx_conn = rx_conn.lock();
        //        let mut buf = Vec::with_capacity(512);
        //        let mut last_pos = 0;
        //        loop {
        //            match rx_conn.read(&mut buf) {
        //                // lol busy wait
        //                Ok(0) => { sleep(Duration::from_secs(1)); }
        //                Ok(n) => {
        //                    let mut stdout = stdout().lock();

        //                    let _ = stdout.write(&buf[last_pos..last_pos + n]);
        //                    if let Err(e) = stdout.flush() {
        //                        eprintln!("Error flushing stdout: {e:?}");
        //                    };

        //                    last_pos += n;
        //                }
        //                Err(e) => { eprintln!("Error on receiving message: {e:?}"); }
        //            }
        //        }
        //    });
        //});

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

            if let Err(e) = self.conn.write_all(msg.0.as_bytes()) {
                eprintln!("Couldn't write message; skipping: {e:?}");
                continue;
            }

            println!("<{}> {}", self.user.name, msg);
        }

        Ok(())
    }
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


#[cfg(test)]
mod tests {
    use std::io::{Cursor, Seek, SeekFrom};
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

    #[test]
    fn test_client_do_auth_flow_success() {
        let user = User::new(String::from("hello"));
        let user_json = serde_json::to_vec(&user).unwrap();

        // Set a response where it _would_ be before the client does any writes
        let mut cursor: Cursor<Vec<u8>> = Default::default();
        cursor.seek(SeekFrom::Start(user_json.len() as u64)).unwrap();
        let _ = cursor.write(&serde_json::to_vec(&AuthResponse::Success).unwrap()).unwrap();
        cursor.seek(SeekFrom::Start(0)).unwrap();

        let mut client = Client::new(user, cursor);
        assert!(client.do_auth_flow().is_ok());
    }

    #[test]
    fn test_client_do_auth_flow_failure() {
        let user = User::new(String::from("hello"));
        let user_json = serde_json::to_vec(&user).unwrap();

        // Set a response where it _would_ be before the client does any writes
        let mut cursor: Cursor<Vec<u8>> = Default::default();
        cursor.seek(SeekFrom::Start(user_json.len() as u64)).unwrap();
        let _ = cursor.write(&serde_json::to_vec(&AuthResponse::Error("".to_string())).unwrap()).unwrap();
        cursor.seek(SeekFrom::Start(0)).unwrap();

        let mut client = Client::new(user, cursor);
        assert!(client.do_auth_flow().is_err());
    }
}
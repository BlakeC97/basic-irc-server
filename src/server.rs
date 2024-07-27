use std::collections::BTreeMap;
use std::fmt::Debug;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, mpsc};
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread;
use parking_lot::Mutex;
use thiserror::Error;
use crate::response::AuthResponse;
use crate::scuffed_clone::ScuffedClone;
use crate::user::User;

pub const VALIDATE_BUFFER_SIZE: usize = 256;
const CHANNEL_SIZE: usize = 128;
type SharedMap<K, V> = Arc<Mutex<BTreeMap<K, V>>>;
type ChatLine = (User, String);

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Failed to read/write from stream: `{0}`")]
    IO(#[from] std::io::Error),
    #[error("Failed serializing user info: `{0}`")]
    Serde(#[from] serde_json::Error),
    #[error("A user is already connected with that name: `{0}`")]
    AlreadyConnected(String),
}

pub fn start(address: SocketAddr) -> std::io::Result<()> {
    let listener = TcpListener::bind(address)?;
    eprintln!("Listening on port {}", listener.local_addr().expect("Can't get local_addr for server").port());

    let connected_users: SharedMap<User, TcpStream> = Default::default();
    let (sender, receiver) = mpsc::sync_channel::<ChatLine>(CHANNEL_SIZE);

    thread::scope(|scope| {
        let users = connected_users.clone();
        scope.spawn(move || { broadcast_messages(users, receiver); });

        for stream_res in listener.incoming() {
            match stream_res {
                Ok(stream) => {
                    let users = connected_users.clone();
                    let tx = sender.clone();
                    scope.spawn(move || handle_connection(stream, users, tx));
                }
                Err(e) => { eprintln!("Failed on handling incoming stream: {e:?}"); }
            }
        }
    });

    Ok(())
}

fn handle_connection<S: Read + Write + ScuffedClone>(
    mut stream: S,
    mut connected_users: SharedMap<User, S>,
    sender: SyncSender<ChatLine>,
) {
    match do_auth_flow(&mut stream, &mut connected_users) {
        Ok(user) => {
            handle_chat(stream, &user, sender);
            connected_users.lock().remove(&user);
        }
        Err(e) => {
            eprintln!("Failed validating user: {e:?}");
        }
    };
}

/// Performs the authorization flow for a connecting user. In addition to the `Result`, this function
/// writes an `AuthResponse` to the stream indicating success or failure.
fn do_auth_flow<S>(stream: &mut S, connected_users: &mut SharedMap<User, S>) -> Result<User, ServerError>
where
    S: Read + Write + ScuffedClone
{
    let mut buf = [0; VALIDATE_BUFFER_SIZE];
    let n = stream.read(&mut buf)?;

    // Don't try to read the null bytes in the buffer
    let user: User = serde_json::from_slice(&buf[..n])?;

    {
        let mut users = connected_users.lock();
        if users.contains_key(&user) {
            let name = user.name.clone();
            let resp = AuthResponse::Error(format!("Name is already taken: {name}"));
            stream.write_all(&serde_json::to_vec(&resp)?)?;
            return Err(ServerError::AlreadyConnected(name));
        }
        users.insert(user.clone(), stream.scuffed_clone());
    }

    stream.write_all(&serde_json::to_vec(&AuthResponse::Success)?)?;
    Ok(user)
}

fn handle_chat<R: Read>(stream: R, user: &User, sender: SyncSender<ChatLine>) {
    let mut buffer = Vec::with_capacity(4096);
    let mut stream = BufReader::with_capacity(4096, stream);
    let mut last_pos = 0;
    let thread_id = format!("[{:?}] ", thread::current().id());

    loop {
        // Basically `read_line` but we want to work with a Vec<u8> directly
        match stream.read_until(0xA, &mut buffer) {
            Ok(n) => {
                if n == 0 {
                    break;
                }

                let s = String::from_utf8_lossy(&buffer[last_pos..last_pos + n])
                    .trim_end()
                    .to_string();
                last_pos += n;

                if let Err(e) = sender.send((user.clone(), s.clone())) {
                    eprintln!("{thread_id} Error sending message: {e:?}");
                }

                eprintln!("{thread_id}<{}> {s:?}", user.name);
            }
            Err(e) => {
                eprintln!("{thread_id}Error reading from stream: {e:?}");
                break;
            }
        }
    }
}

fn broadcast_messages<S>(users: SharedMap<User, S>, receiver: Receiver<ChatLine>)
where
    S: Read + Write + ScuffedClone
{
    for (user, msg) in receiver {
        let full_msg = format!("<{user}> {msg}").into_bytes();

        users
            .lock()
            .iter_mut()
            .filter(|(u, _)| *u != &user)
            .for_each(|(u, conn)| {
                if let Err(e) = conn.write_all(&full_msg) {
                    eprintln!("[BROADCAST] Failed sending message to {u}: {e:?}");
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use super::*;

    #[test]
    fn do_auth_flow_valid_json() {
        let user = User::new("hello");
        let user_json = serde_json::to_vec(&user).unwrap();
        let mut expected_cursor = {
            let mut v: Vec<u8> = Vec::new();
            v.extend(&user_json);
            v
        };

        let mut cursor = Cursor::new(user_json);

        let success_resp = serde_json::to_vec(&AuthResponse::Success).unwrap();
        expected_cursor.extend(&success_resp);

        assert_eq!(user, do_auth_flow(&mut cursor, &mut Default::default()).unwrap());
        assert_eq!(&expected_cursor, cursor.get_ref());
    }

    // Only necessary because of VALIDATE_BUFFER_SIZE
    #[test]
    fn do_auth_flow_buffer_length_failure() {
        let mut long_str = String::with_capacity(VALIDATE_BUFFER_SIZE);
        (0..VALIDATE_BUFFER_SIZE).for_each(|_| long_str.push('a'));
        let user = User::new(long_str.clone());
        let user_json = serde_json::to_vec(&user).unwrap();
        let user_json_len = user_json.len();

        let mut cursor = Cursor::new(user_json.clone());

        let res = do_auth_flow(&mut cursor, &mut Default::default()).err().unwrap();
        // Force a Serde error since idk how to manually create one
        let se = serde_json::from_slice::<User>(&cursor.get_ref()[..user_json_len - 1]).err().unwrap();
        assert_eq!(
            std::mem::discriminant(&res),
            std::mem::discriminant(&ServerError::Serde(se))
        );
        assert_eq!(&user_json, cursor.get_ref());
    }

    #[test]
    fn do_auth_flow_already_logged_in() {
        let user = User::new("hello");
        let user_json = serde_json::to_vec(&user).unwrap();
        let mut expected_cursor = {
            let mut l: Vec<u8> = Vec::new();
            l.extend(&user_json);
            l
        };
        let mut cursor = Cursor::new(user_json);

        let mut connected_users: SharedMap<User, _> = Default::default();
        {
            connected_users.lock().insert(user.clone(), cursor.scuffed_clone());
        }

        let failure_res = serde_json::to_vec(&AuthResponse::Error("Name is already taken: hello".to_string())).unwrap();
        expected_cursor.extend(failure_res);

        let res = do_auth_flow(&mut cursor, &mut connected_users).err().unwrap();
        assert_eq!(
            std::mem::discriminant(&res),
            std::mem::discriminant(&ServerError::AlreadyConnected("".to_string()))
        );
        assert_eq!(&expected_cursor, cursor.get_ref());
    }

    #[test]
    fn broadcast_message() {
        let user_1 = User::new("one");
        let user_2 = User::new("two");

        let connected_users: SharedMap<User, _> = Default::default();
        connected_users.lock().insert(user_1.clone(), Cursor::new(Vec::<u8>::new()));
        connected_users.lock().insert(user_2.clone(), Cursor::new(Vec::<u8>::new()));

        let (tx, rx) = mpsc::sync_channel::<ChatLine>(CHANNEL_SIZE);
        tx.send((user_1.clone(), "hello".to_string())).unwrap();
        tx.send((user_2.clone(), "yo waddup".to_string())).unwrap();
        drop(tx);

        broadcast_messages(connected_users.clone(), rx);
        {
            let users = connected_users.lock();
            assert_eq!(Cursor::new(Vec::from(b"<two> yo waddup")).get_ref(), users.get(&user_1).unwrap().get_ref());
            assert_eq!(Cursor::new(Vec::from(b"<one> hello")).get_ref(), users.get(&user_2).unwrap().get_ref());
        }
    }
}
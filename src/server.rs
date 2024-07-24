use std::io::{BufRead, BufReader, Read};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::thread;
use crate::user::User;

const VALIDATE_BUFFER_SIZE: usize = 128;

pub fn start(port: u16) -> std::io::Result<()> {
    let listener = {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
        let l = TcpListener::bind(addr)?;
        eprintln!("Listening on port {}", l.local_addr().expect("bind would have returned on error").port());

        l
    };

    thread::scope(|scope| {
        for stream_res in listener.incoming() {
            match stream_res {
                Ok(mut stream) => {
                    scope.spawn(move || {
                        match validate_user(&mut stream) {
                            Ok(user) => {
                                handle_connection(stream, user);
                            }
                            Err(e) => { eprintln!("Failed validating user: {e:?}"); }
                        };
                    });
                }
                Err(e) => { eprintln!("Failed on handling incoming TcpStream: {e:?}"); }
            }
        }
    });

    Ok(())
}

fn validate_user<R: Read>(stream: &mut R) -> std::io::Result<User> {
    let mut buf = [0; VALIDATE_BUFFER_SIZE];
    let n = stream.read(&mut buf)?;

    // Don't try to read the null bytes in the buffer
    Ok(serde_json::from_slice(&buf[..n])?)
}

fn handle_connection<R: Read>(stream: R, user: User) {
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

                eprintln!("{thread_id}<{}> {s:?}", user.name);
            }
            Err(e) => {
                eprintln!("{thread_id}Error reading from TcpStream: {e:?}");
                break;
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use super::*;

    #[test]
    fn validate_user_valid_json() {
        let user = User::new("hello");
        let user_json = serde_json::to_string(&user).unwrap();

        assert_eq!(user, validate_user(&mut Cursor::new(user_json)).unwrap());
    }

    // Only necessary because of VALIDATE_BUFFER_SIZE
    #[test]
    fn validate_user_buffer_length_failure() {
        let mut long_str = String::with_capacity(VALIDATE_BUFFER_SIZE);
        (0..VALIDATE_BUFFER_SIZE).for_each(|_| long_str.push('a'));
        let user = User::new(long_str);
        let user_json = serde_json::to_string(&user).unwrap();

        assert!(validate_user(&mut Cursor::new(user_json)).is_err());
    }
}
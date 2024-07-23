use std::io::{BufRead, BufReader};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::thread;

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
                Ok(stream) => {
                    scope.spawn(move || { handle_connection(stream); });
                }
                Err(e) => { eprintln!("Failed on handling incoming TcpStream: {e:?}"); }
            }
        }
    });

    Ok(())
}

fn handle_connection(stream: TcpStream) {
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

                // Minus one to strip the newline
                let s = String::from_utf8_lossy(&buffer[last_pos..last_pos + n - 1]);
                eprintln!("{thread_id}Read {n} bytes; as a string: {s:?}");
                last_pos += n;
            }
            Err(e) => {
                eprintln!("{thread_id}Error reading from TcpStream: {e:?}");
                break;
            }
        }
    }

    eprintln!("{thread_id}Full buffer: {buffer:?}");
}
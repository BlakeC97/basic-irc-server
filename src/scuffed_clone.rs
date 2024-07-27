use std::io::Cursor;
use std::net::TcpStream;

// So I can use TcpStream for real, but an std::io::Cursor in testing
pub trait ScuffedClone {
    fn scuffed_clone(&self) -> Self;
}

impl ScuffedClone for TcpStream {
    fn scuffed_clone(&self) -> Self {
        self.try_clone().expect("Scuffed clone on a TcpStream didn't work, lolrip")
    }
}

impl<T: Clone> ScuffedClone for Cursor<T> {
    fn scuffed_clone(&self) -> Self {
        self.clone()
    }
}


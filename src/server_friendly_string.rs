use std::fmt::{Display, Formatter};

/// A String that's guaranteed to end with a line feed (LF, '\n', 0xA)
/// and trims whitespace from the end of the string
/// for the sake of Client/Server communication + stdin shenanigans.
///
/// The various methods on it (`len` etc.) are meant to remove the line feed from its calculations,
/// e.g. `ServerFriendlyString.len()` returns the length of the String _without_ the line feed.
/// If one wants the String _with_ a line feed, access the underlying data with `self.0`.
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
pub struct ServerFriendlyString(pub String);

impl ServerFriendlyString {
    /// Returns the length of the String without the line feed character.
    pub fn len(&self) -> usize {
        self.0.len() - 1
    }
}

impl<T: Into<String>> From<T> for ServerFriendlyString {
    fn from(value: T) -> Self {
        let mut value = value.into().trim_end().to_string();
        value.push('\n');
        Self(value)
    }
}

impl Display for ServerFriendlyString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0[..self.len()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_friendly_string_no_newline() {
        let input = "hello world";
        let sfs = ServerFriendlyString::from(input);
        assert_eq!(input.len(), sfs.len());
        assert_eq!(input, format!("{sfs}"));
    }

    #[test]
    fn test_server_friendly_string_whitespace() {
        let input = "hello world\r\n";
        let sfs = ServerFriendlyString::from(input);
        assert_eq!(input.len() - 2, sfs.len());
        assert_eq!("hello world", format!("{sfs}"));

        let input = "hello world\r";
        let sfs = ServerFriendlyString::from(input);
        assert_eq!(input.len() - 1, sfs.len());
        assert_eq!("hello world", format!("{sfs}"));

        let input = "hello world\t";
        let sfs = ServerFriendlyString::from(input);
        assert_eq!(input.len() - 1, sfs.len());
        assert_eq!("hello world", format!("{sfs}"));

        let input = "hello world\n\t\n";
        let sfs = ServerFriendlyString::from(input);
        assert_eq!(input.len() - 3, sfs.len());
        assert_eq!("hello world", format!("{sfs}"));
    }
}
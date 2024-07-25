use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub struct User {
    pub name: String,
}

impl User {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into()
        }
    }
}

use std::{fmt::Display, io::Error, str::FromStr};

use crate::to_io_result::err;

#[derive(Debug, PartialEq, Clone, Copy, Eq)]
pub enum Method {
    GET,
    POST,
}

const GET: &str = "GET";
const POST: &str = "POST";

/// See https://www.rfc-editor.org/rfc/rfc9110.html#section-9.1-5
impl Method {
    pub const fn to_str(self) -> &'static str {
        match self {
            Method::GET => GET,
            Method::POST => POST,
        }
    }
}

impl FromStr for Method {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // HTTP methods are case-sensitive.
        match s {
            GET => Ok(Self::GET),
            POST => Ok(Self::POST),
            _ => err("unknown HTTP method"),
        }
    }
}

impl Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

#[cfg(test)]
mod tests {
    use super::Method;

    #[test]
    fn display() {
        assert_eq!(format!("method: {}", Method::GET), "method: GET");
        assert_eq!(format!("method: {}", Method::POST), "method: POST");
    }
}

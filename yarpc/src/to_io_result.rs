use std::io::{Error, ErrorKind};

pub trait ToIoResult {
    type V;
    fn to_io_result(self) -> Result<Self::V, Error>;
}

fn error<E: Into<Box<dyn std::error::Error + Send + Sync>>>(e: E) -> Error {
    Error::new(ErrorKind::InvalidData, e)
}

pub fn err<T, E: Into<Box<dyn std::error::Error + Send + Sync>>>(e: E) -> Result<T, Error> {
    Err(error(e))
}

impl<T> ToIoResult for Option<T> {
    type V = T;
    fn to_io_result(self) -> Result<Self::V, Error> {
        self.map_or(err("option"), Ok)
    }
}

impl<T, E: Into<Box<dyn std::error::Error + Send + Sync>>> ToIoResult for Result<T, E> {
    type V = T;
    fn to_io_result(self) -> Result<Self::V, Error> {
        self.or_else(err)
    }
}

pub trait TakeToIoResult {
    type V;
    fn take_to_io_result(&mut self) -> Result<Self::V, Error>;
}

impl<T> TakeToIoResult for Option<T> {
    type V = T;
    fn take_to_io_result(&mut self) -> Result<T, Error> {
        self.take().to_io_result()
    }
}

#[cfg(test)]
mod tests {
    use super::ToIoResult;

    #[test]
    fn option() {
        let e: Result<u8, String> = Err("hello".to_string());
        let r = e.to_io_result();
        let x = format!("{:?}", r);
        assert_eq!(
            x,
            "Err(Custom { kind: InvalidData, error: \"hello\" })".to_string()
        );
    }
}

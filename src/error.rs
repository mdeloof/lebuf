#[derive(Debug)]
pub enum Error {
    /// Failed to write whole buffer.
    WriteZero,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::WriteZero => write!(f, "WriteZero"),
        }
    }
}

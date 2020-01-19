
pub type AppResult<T> = std::result::Result<T, AppError>;


#[derive(Fail, Debug)]
pub enum AppError {
    #[fail(display = "IO Error: {}", _0)]
    IOError(String),

    #[fail(display = "Decode Error: {}", _0)]
    Decode(String),


    #[fail(display = "Error: {}", _0)]
    AppError(String),
}


impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> AppError {
        AppError::IOError(err.to_string())
    }
}

impl From<std::str::Utf8Error> for AppError {
    fn from(err: std::str::Utf8Error) -> AppError {
        AppError::Decode(err.to_string())
    }
}

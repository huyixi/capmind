use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    InvalidInput(String),
    #[error("{0}")]
    Auth(String),
    #[error("{0}")]
    Api(String),
    #[error("{0}")]
    Network(String),
}

impl AppError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::InvalidInput(_) => 2,
            Self::Auth(_) => 3,
            Self::Api(_) => 4,
            Self::Network(_) => 5,
        }
    }
}

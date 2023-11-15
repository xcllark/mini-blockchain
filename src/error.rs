use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("I/O Error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("K256 Error: {0}")]
    K256Error(#[from] k256::ecdsa::Error),

    #[error("Couldn't read entire message from the socket")]
    IncompleteMessage,

    #[error("Connection ended by peer")]
    ConnectionEnded,

    #[error("Tokio Join Error: {0}")]
    TokioJoinError(#[from] tokio::task::JoinError),

    #[error("System time error: {0}")]
    SystemTimeError(#[from] std::time::SystemTimeError),

    #[error("Channel failure")]
    ChannelFailure,
}

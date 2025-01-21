use thiserror::Error;
use solana_client::client_error::ClientError;

#[derive(Clone, Debug, PartialEq)]
pub enum TradeSubmissionError {
    #[error("Insufficient balance: required {required} SOL, available {available} SOL")]
    InsufficientBalance {
        required: f64,
        available: f64,
    },

    #[error("RPC Client Error: {0}")]
    RpcError(String),

    #[error("Transaction submission timeout")]
    SubmissionTimeout,

    #[error("Transaction confirmation failed: {0}")]
    ConfirmationError(String),

    #[error("Slippage protection triggered")]
    SlippageProtection,

    #[error("Network congestion detected")]
    NetworkCongestion,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExecutionError {
    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Timeout error after {0:?}")]
    TimeoutError(Duration),

    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance {
        required: f64,
        available: f64,
    },

    #[error("Order validation failed: {0}")]
    ValidationError(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Position not found: {0}")]
    PositionNotFound(String),

    #[error("Slippage exceeded: expected {expected}, actual {actual}")]
    SlippageExceeded {
        expected: f64,
        actual: f64,
    },

    #[error("Price impact too high: {0}%")]
    HighPriceImpact(f64),

    #[error("Order rejected: {0}")]
    OrderRejected(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionErrorType {
    RpcError,
    NetworkError,
    TimeoutError,
    InsufficientBalance,
    ValidationError,
    TransactionError,
    PositionNotFound,
    SlippageExceeded,
    HighPriceImpact,
    OrderRejected,
}

impl ExecutionError {
    pub fn error_type(&self) -> ExecutionErrorType {
        match self {
            ExecutionError::RpcError(_) => ExecutionErrorType::RpcError,
            ExecutionError::NetworkError(_) => ExecutionErrorType::NetworkError,
            ExecutionError::TimeoutError(_) => ExecutionErrorType::TimeoutError,
            ExecutionError::InsufficientBalance { .. } => ExecutionErrorType::InsufficientBalance,
            ExecutionError::ValidationError(_) => ExecutionErrorType::ValidationError,
            ExecutionError::TransactionError(_) => ExecutionErrorType::TransactionError,
            ExecutionError::PositionNotFound(_) => ExecutionErrorType::PositionNotFound,
            ExecutionError::SlippageExceeded { .. } => ExecutionErrorType::SlippageExceeded,
            ExecutionError::HighPriceImpact(_) => ExecutionErrorType::HighPriceImpact,
            ExecutionError::OrderRejected(_) => ExecutionErrorType::OrderRejected,
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self.error_type(),
            ExecutionErrorType::RpcError
                | ExecutionErrorType::NetworkError
                | ExecutionErrorType::TimeoutError
        )
    }
}
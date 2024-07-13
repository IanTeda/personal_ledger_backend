//-- ./src/errors.rs

// #![allow(unused)] // For beginning only.

//! Main Crate Error
//! # References
//!
//! * [Rust Error Handling - Best Practices](https://www.youtube.com/watch?v=j-VQCYP7wyw)
//! * [jeremychone-channel/rust-base](https://github.com/jeremychone-channel/rust-base)
//! * [derive(Error)](https://github.com/dtolnay/thiserror)
//! * [How to Handle Errors in Rust: A Comprehensive Guide](https://dev.to/nathan20/how-to-handle-errors-in-rust-a-comprehensive-guide-1cco)
//! * [Rust Error Types Explained: Building Robust Error Handling](https://marketsplash.com/rust-error-types/)

use tonic::Status;

/// Static errors types
#[derive(thiserror::Error, Debug)]
pub enum BackendError {
	//-- Generic Errors
	/// For starter, to remove as code matures.
	#[error("Generic error: {0}")]
	Generic(String),
	/// For starter, to remove as code matures.
	#[error("Static error: {0}")]
	Static(&'static str),

	//-- Module errors
	#[error("Email address was empty")]
	EmailIsEmpty,

	#[error("Email format is invalid: {0}")]
	EmailFormatInvalid(String),

	#[error("Name format is invalid: {0}")]
	UserNameFormatInvalid(String),

	#[error("Password does not meet minimum requirements")]
	PasswordFormatInvalid,

	#[error("Password parsing error")]
	PasswordParseError,

	#[error("Authentication error: {0}")]
	AuthenticationError(String),

	//-- External errors
	/// Derive IO errors
	#[error(transparent)]
	IO(#[from] std::io::Error),
	// Config errors
	#[error(transparent)]
	Config(#[from] config::ConfigError),
	// Tonic Reflections errors
	#[error(transparent)]
	TonicReflection(#[from] tonic_reflection::server::Error),
	// Tonic transport errors
	#[error(transparent)]
	TonicTransport(#[from] tonic::transport::Error),
	// Standard network address error
	#[error(transparent)]
	AddressParse(#[from] std::net::AddrParseError),

	// Environmental parse error
	// #[error(transparent)]
	// EnvironmentParse(#[from] std::env::VarError),
	#[error(transparent)]
	LogError(#[from] tracing_log::log::SetLoggerError),

	#[error(transparent)]
	TracingError(#[from] tracing::dispatcher::SetGlobalDefaultError),

	#[error(transparent)]
	SqlxMigration(#[from] sqlx::migrate::MigrateError),
	// sqlx::migrate::MigrateError
	#[error(transparent)]
	Sqlx(#[from] sqlx::Error),
	// Parsing String to UUid
	#[error("Uuid: {0}")]
	Uuid(#[from] uuid::Error),
	// #[error(transparent)]
	// Argon2(#[from] argon2::password_hash::Error),
	#[error("json: {0}")]
	Json(#[from] serde_json::Error),

	#[error("json web token: {0}")]
	JsonWebToken(#[from] jsonwebtoken::errors::Error),
}

impl From<BackendError> for tonic::Status {
	fn from(backend_error: BackendError) -> tonic::Status {
		match backend_error {
			BackendError::AuthenticationError(m) => Status::unauthenticated(m),
			// BackendError::EmailFormatInvalid(_) => {
			//     Status::invalid_argument(format!("{:?}", backend_error))
			// }
			//  BackendError::Uuid(_) => {
			// 	Status::internal("Internal server error")
			//  }
			// BackendError::InvalidUsernameOrPassword => {
			//     tonic::Status::unauthenticated(format!("{:?}", backend_error))
			// }
			// BackendError::UserAlreadyExists(_) => {
			//     tonic::Status::invalid_argument(format!("{:?}", backend_error))
			// }
			// BackendError::DatabaseError(_) => tonic::Status::unavailable(format!("{:?}", backend_error)),
			// BackendError::InvalidToken(_) => {
			//     tonic::Status::unauthenticated(format!("{:?}", backend_error))
			// }
			// BackendError::HashingError => tonic::Status::unavailable(format!("{:?}", backend_error)),
			// _ => tonic::Status::unknown(format!("{:?}", backend_error)),
			_ => Status::internal("Internal server error"),
		}
	}
}

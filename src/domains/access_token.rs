//-- ./src/domains/access_token.rs

// #![allow(unused)] // For beginning only.

//! JSON Web Token used to authorise RPC endpoint requests
//!
//! Generate a new instance of Access Token and decode an existing Access Token
//! into a Token Claim
//! ---

use crate::{domains::token_claim::TokenType, prelude::*};

use jsonwebtoken::{
	decode, encode, errors::Error as JWTError, DecodingKey, EncodingKey, Header,
	Validation,
};
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{TokenClaim, TOKEN_ISSUER};

pub static ACCESS_TOKEN_DURATION: u64 = 5 * 60; // 15 minutes as seconds

/// Access Token for authorising endpoint requests
pub struct AccessToken(String);

/// Get string reference of the Access Token
impl AsRef<str> for AccessToken {
	fn as_ref(&self) -> &str {
		&self.0
	}
}

/// Roll our own Display trait for Access Token
impl std::fmt::Display for AccessToken {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

impl AccessToken {
	/// Parse a new Access Token, returning a Result with an AccessToken or BackEnd error
	///
	/// ## Parameters
	///
	/// * `secret`: Secret<String> containing the token encryption secret
	/// * `user_id`: Uuid of the user that is going to use the Access Token
	/// ---
	#[tracing::instrument(
		name = "Generate a new Access Token for: "
		skip(secret)
		// fields(
        // 	db_id = %self.id,
		// 	user_id = %self.user_id,
		// 	refresh_token = %self.refresh_token.as_ref(),
		// 	is_active = %self.is_active,
		// 	created_on = %self.created_on,
    	// )
	)]
	pub async fn new(
		secret: &Secret<String>,
		user_id: &Uuid,
	) -> Result<Self, BackendError> {
		// Convert Uuid into a String
		let user_id = user_id.to_string();

		// Build the Access Token Claim
		let token_claim = TokenClaim::new(&secret, &user_id, &TokenType::Access);

		// Encode the Token Claim into a URL-safe hash encryption
		let token = encode(
			&Header::default(),
			&token_claim,
			&EncodingKey::from_secret(secret.expose_secret().as_bytes()),
		)?;

		Ok(Self(token))
	}
}

#[cfg(test)]
mod tests {

	use crate::database::UserModel;

	// Bring module into test scope
	use super::*;

	use rand::distributions::{Alphanumeric, DistString};

	// Override with more flexible error
	pub type Result<T> = core::result::Result<T, Error>;
	pub type Error = Box<dyn std::error::Error>;

	#[tokio::test]
	async fn generate_new_access_token() -> Result<()> {
		// Generate random secret string
		let secret = Alphanumeric.sample_string(&mut rand::thread_rng(), 60);
		let secret = Secret::new(secret);

		// Get a random user_id for subject
		let random_user = UserModel::mock_data().await?;

		let access_token = AccessToken::new(&secret, &random_user.id).await?;

		let token_claim =
			TokenClaim::from_token(access_token.as_ref(), &secret).await?;
		// println!("{token_claim:#?}");

		assert_eq!(token_claim.iss, TOKEN_ISSUER);
		assert_eq!(token_claim.sub, random_user.id.to_string());
		assert_eq!(token_claim.jty, TokenType::Access.to_string());

		Ok(())
	}
}

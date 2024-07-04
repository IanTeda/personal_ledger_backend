//-- ./src/rpc/auth.rs

//! Return a result containing a RPC Users service

#![allow(unused)] // For development only

use crate::database;
use crate::database::users::update_password_by_id;
use crate::domains::{verify_password_hash, EmailAddress, Password};

use secrecy::Secret;
use sqlx::{Pool, Postgres};
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::rpc::ledger::auth_server::Auth;
use crate::rpc::ledger::{
	AuthenticateRequest, AuthenticateResponse, Empty, LogoutRequest, ResetPasswordRequest,
	ResetPasswordResponse, UpdatePasswordRequest,
};

// /// User service containing a database pool
#[derive(Debug)]
pub struct AuthService {
	database: Pool<Postgres>,
}

impl AuthService {
	pub fn new(database: Pool<Postgres>) -> Self {
		Self { database }
	}
}

#[tonic::async_trait]
impl Auth for AuthService {
	async fn authenticate(
		&self,
		request: Request<AuthenticateRequest>,
	) -> Result<Response<AuthenticateResponse>, Status> {
		let request = request.into_inner();
		let email = EmailAddress::parse(&request.email)?;
		let password = Secret::new(request.password);

		let user = database::users::select_user_by_email(&email, &self.database).await?;

		match verify_password_hash(&password, user.password_hash.as_ref())? {
			true => {
				let response = AuthenticateResponse {
					token: "Super-Secret-Token".to_string(),
				};
				Ok(Response::new(response))
			}
			false => Err(Status::unauthenticated("Authentication failed!")),
		}
		// unimplemented!()
	}

	async fn update_password(
		&self,
		request: Request<UpdatePasswordRequest>,
	) -> Result<Response<AuthenticateResponse>, Status> {
		let request = request.into_inner();
		let email = EmailAddress::parse(request.email)?;
		let original_password = Secret::new(request.original_password);
		let new_password = Secret::new(request.new_password);

		let user = database::users::select_user_by_email(&email, &self.database).await?;

		match verify_password_hash(&original_password, user.password_hash.as_ref())? {
			true => {
				let new_password_hash = Password::parse(new_password)?;
				let is_updated =
					update_password_by_id(user.id, new_password_hash, &self.database).await?;

				let response = AuthenticateResponse {
					token: "Super-Secret-Token".to_string(),
				};

				Ok(Response::new(response))
			}
			false => Err(Status::unauthenticated("Authentication failed!")),
		}

		// unimplemented!()
	}

	async fn reset_password(
		&self,
		request: Request<ResetPasswordRequest>,
	) -> Result<Response<ResetPasswordResponse>, Status> {
		unimplemented!()
	}

	async fn logout(&self, request: Request<LogoutRequest>) -> Result<Response<Empty>, Status> {
		unimplemented!()
	}
}

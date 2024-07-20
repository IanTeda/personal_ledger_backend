//-- ./src/rpc/auth.rs

//! Return a result containing an RPC Authentication Service

#![allow(unused)] // For development only

use std::sync::Arc;

use secrecy::Secret;
use sqlx::{Pool, Postgres};
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::{database, domain};
use crate::configuration::Configuration;
use crate::prelude::*;
use crate::rpc::proto::{LoginRequest, LogoutRequest, LogoutResponse, RefreshRequest, RegisterRequest, ResetPasswordRequest, ResetPasswordResponse, TokenResponse, UpdatePasswordRequest};
use crate::rpc::proto::authentication_server::Authentication;

// use crate::rpc::proto::authentication_server::Authentication;
// use crate::rpc::proto::LoginRequest;

/// Authentication service containing a database pool
pub struct AuthenticationService {
    /// Database Arc reference
    database: Arc<Pool<Postgres>>,
    /// Configuration Arc reference
    config: Arc<Configuration>,
}

impl AuthenticationService {
    /// Initiate a new Authentication Service
    pub fn new(database: Arc<Pool<Postgres>>, config: Arc<Configuration>) -> Self {
        Self { database, config }
    }

    /// Shorthand reference to database pool
    fn database_ref(&self) -> &Pool<Postgres> {
        &self.database
    }

    /// Shorthand reference to config
    fn config_ref(&self) -> &Configuration {
        &self.config
    }
}

#[tonic::async_trait]
impl Authentication for AuthenticationService {
    #[tracing::instrument(name = "Authenticate Request: ", skip(self, request))]
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<TokenResponse>, Status> {
        // Break up the request into its three parts: 1. Metadata, 2. Extensions & 3. Message
        let (_request_metadata, _request_extensions, request_message) =
            request.into_parts();

        // Parse the request email string into an EmailAddress
        let request_email = domain::EmailAddress::parse(&request_message.email)
            .map_err(|_| {
                BackendError::AuthenticationError(
                    "Authentication failed!".to_string(),
                )
            })?;

        tracing::debug!("Request email: {}", request_email.as_ref());

        // Get the user from the database using the request email, so we can verify password hash
        let user =
            database::Users::from_user_email(&request_email, self.database_ref())
                .await
                .map_err(|_| {
                    tracing::error!(
                        "User email not found in database: {}",
                        request_email.as_ref()
                    );
                    BackendError::AuthenticationError(
                        "Authentication Failed!".to_string(),
                    )
                })?;

        tracing::debug!("User retrieved from the database: {}", user.id);

        // Wrap the Token Secret string in a Secret
        let token_secret = Secret::new(self.config.application.token_secret.clone());

        // Wrap request password in a Secret
        let password_secret = Secret::new(request_message.password);

        // Check password against stored hash
        match user.password_hash.verify_password(&password_secret)? {
            true => {
                tracing::info!("Password verified.");

                // Build an Access Token
                let access_token = domain::AccessToken::new(&token_secret, &user)?;

                tracing::debug!("Using Access Token: {}", access_token);

                // Build a Refresh Token
                let refresh_token =
                    database::RefreshTokens::new(&user, &token_secret)?;

                // Add Refresh Token to database
                let refresh_token =
                    refresh_token.insert(self.database_ref()).await?;

                tracing::debug!("Using Refresh Token: {}", refresh_token.token);

                // Build Authenticate Response with the token
                let response = TokenResponse {
                    access_token: access_token.to_string(),
                    refresh_token: refresh_token.token.to_string(),
                };

                // Send Response
                Ok(Response::new(response))
            }
            false => {
                tracing::error!("Password verification failed.");
                Err(Status::unauthenticated("Authentication Failed!"))
            }
        }
    }

    /// Get a new Access Token using the Refresh Token that has a longer life
    #[tracing::instrument(
        name = "Refresh Access Token Request: ",
        skip(self, request)
    )]
    async fn refresh(
        &self,
        request: Request<RefreshRequest>,
    ) -> Result<Response<TokenResponse>, Status> {
        // Break up the request into its three parts: 1. Metadata, 2. Extensions & 3. Message
        let (_request_metadata, _request_extensions, request_message) =
            request.into_parts();

        //-- 1. Get the Refresh Token
        // Get the RefreshAuthenticationRequest from inside the Tonic Request
        // let request = request.into_inner();
        let refresh_token = request_message.refresh_token;

        //-- 2. Get & Validate  the Refresh Token Claim
        // Get the Token Secret from config and wrap it in a Secret to help limit leaks
        let token_secret = &self.config_ref().application.token_secret;
        let token_secret = Secret::new(token_secret.to_owned());

        // Using the Token Secret decode the token into a Token Claim
        // This also validates the token expiration, not before and Issuer
        let refresh_token_claim = domain::TokenClaim::from_token(
            &refresh_token,
            &token_secret,
        )
            .map_err(|_| {
                tracing::error!("Refresh Token is invalid!");
                BackendError::AuthenticationError("Authentication Failed!".to_string())
            })?;

        //-- 3. Check Refresh Token status in database
        let database_record =
            database::RefreshTokens::from_token(&refresh_token, self.database_ref())
                .await?;

        match database_record.is_active {
            true => {
                tracing::info!("Refresh Token is active.");

                //-- 4. Void all Refresh Tokens for associated user ID
                database_record
                    .revoke_associated(self.database_ref())
                    .await?;

                let user_id =
                    Uuid::try_parse(&refresh_token_claim.sub).map_err(|_| {
                        tracing::error!("Unable to parse Uuid");
                        BackendError::AuthenticationError(
                            "Authentication Failed!".to_string(),
                        )
                    })?;

                let user =
                    database::Users::from_user_id(&user_id, self.database_ref())
                        .await?;

                //-- 5. Generate new Access and Refresh Tokens
                // Build an Access Token
                let access_token = domain::AccessToken::new(&token_secret, &user)?;

                tracing::debug!("Using Access Token: {}", access_token);

                // Build a Refresh Token
                let refresh_token =
                    database::RefreshTokens::new(&user, &token_secret)?;

                // Add Refresh Token to database
                let refresh_token =
                    refresh_token.insert(self.database_ref()).await?;

                tracing::debug!("Using Refresh Token: {}", refresh_token.token);

                //-- 5. Send new Access Token and Refresh Token
                // Build Authenticate Response with the token
                let response = TokenResponse {
                    access_token: access_token.to_string(),
                    refresh_token: refresh_token.token.to_string(),
                };

                // Send Response
                Ok(Response::new(response))
                // unimplemented!()
            }
            false => {
                tracing::error!("Refresh Token is not active");
                Err(Status::unauthenticated("Authentication Failed!"))
            }
        }
        //
    }

    #[tracing::instrument(name = "Update Password Request: ", skip(self, request))]
    async fn update_password(
        &self,
        request: Request<UpdatePasswordRequest>,
    ) -> Result<Response<TokenResponse>, Status> {
        //-- 0. Break the request up into its parts
        let (request_metadata, _extensions, request_message) = request.into_parts();

        //-- 1. Get access token and verify
        // Get Access Token from the request
        let access_token = request_metadata
            .get("access_token")
            .ok_or(BackendError::AuthenticationError(
                "Authentication Failed! 1".to_string(),
            ))?
            .to_str()
            .map_err(|_| {
                tracing::error!("Unable to parse access token from header!");
                BackendError::AuthenticationError(
                    "Authentication Failed! 2".to_string(),
                )
            })?;
        tracing::debug!("Using Access Token: {}", access_token);

        // Get the Token Secret from config and wrap it in a Secret to help limit leaks
        let token_secret = &self.config_ref().application.token_secret;
        let token_secret = Secret::new(token_secret.to_owned());

        // Using the Token Secret decode the Access Token into a Token Claim. This also
        // validates the token expiration, not before and Issuer.
        let access_token_claim =
            domain::TokenClaim::from_token(&access_token, &token_secret).map_err(
                |_| {
                    tracing::error!("Access Token is invalid!");
                    return BackendError::AuthenticationError(
                        "Authentication Failed!".to_string(),
                    );
                },
            )?;
        // tracing::debug!("Decoded Access Token Claim: {}", access_token_claim);

        //-- 2. Get user from database and check status
        // We can only change our own password so use the user_id in the access token
        // Parse token claim user_id string into a UUID
        let user_id: Uuid = access_token_claim.sub.parse().map_err(|_| {
            tracing::error!("Unable to parse user id to UUID!");
            return BackendError::AuthenticationError(
                "Authentication Failed!".to_string(),
            );
        })?;

        // Get the user from the database using the token claim user_id, so we
        // can verify status and password hash
        let mut user = database::Users::from_user_id(&user_id, self.database_ref())
            .await
            .map_err(|_| {
                tracing::error!("User id not found in database: {}", user_id);
                return BackendError::AuthenticationError(
                    "Authentication Failed!".to_string(),
                );
            })?;

        // Check user is active
        if user.is_active == false {
            tracing::error!("User is not active: {}", user_id);
            return Err(Status::unauthenticated("Authentication Failed!"));
        }
        tracing::debug!("User is active in the database: {}", user.id);

        // Check user email is verified
        if user.is_verified == false {
            tracing::error!("User email is not verified: {}", user_id);
            return Err(Status::unauthenticated("Authentication Failed!"));
        }
        tracing::debug!("User email is verified in the database: {}", user.id);

        //-- 4. Verify existing/original password
        let original_password = Secret::new(request_message.password_original);
        if user.password_hash.verify_password(&original_password)? == false {
            tracing::error!("Original password is incorrect");
            return Err(Status::unauthenticated("Authentication Failed!"));
        }
        tracing::debug!("Users original password is verified: {}", user.id);

        //-- 5. Update the users password in the database
        let new_password = Secret::new(request_message.password_new);
        let new_password_hash = domain::PasswordHash::parse(new_password)?;
        user.password_hash = new_password_hash;
        user.update(&self.database_ref());
        tracing::debug!("Users password updated in the database: {}", user.id);

        // Build an new Access Token
        let access_token = domain::AccessToken::new(&token_secret, &user)?;
        tracing::debug!("Using Access Token: {}", access_token);

        // Build a new Refresh Token
        let refresh_token = database::RefreshTokens::new(&user, &token_secret)?;

        // Revoke refresh tokens associated with the user before adding new one to the database
        // TODO: When do we clean up (delete) the database
        let _rows_affected = refresh_token
            .revoke_associated(self.database_ref())
            .await?;

        // Add new Refresh Token to the database
        let refresh_token = refresh_token.insert(self.database_ref()).await?;
        tracing::debug!("Using Refresh Token: {}", refresh_token.token);

        // Build Token Response message with the token
        let response_message = TokenResponse {
            access_token: access_token.to_string(),
            refresh_token: refresh_token.token.to_string(),
        };

        // Send Response
        Ok(Response::new(response_message))
    }

    #[tracing::instrument(name = "Reset Password Request: ", skip(self, request))]
    async fn reset_password(
        &self,
        request: Request<ResetPasswordRequest>,
    ) -> Result<Response<ResetPasswordResponse>, Status> {
        //-- 0. Break the request up into its parts
        let (metadata, _extensions, request_message) = request.into_parts();

        unimplemented!()
    }

    #[tracing::instrument(name = "Register User Request: ", skip(self, request))]
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<TokenResponse>, Status> {
        //-- 0. Break the request up into its parts
        let (metadata, _extensions, request_message) = request.into_parts();

        unimplemented!()
    }

    /// Revoke all Refresh Tokens in the database
    #[tracing::instrument(name = "Log Out User Request: ", skip(self, request))]
    async fn logout(
        &self,
        request: Request<LogoutRequest>,
    ) -> Result<Response<LogoutResponse>, Status> {
        // Break up the request into its three parts: 1. Metadata, 2. Extensions & 3. Message
        let (_request_metadata, _request_extensions, request_message) =
            request.into_parts();

        //-- 1. Get the Refresh Token
        // Get the RefreshAuthenticationRequest from inside the Tonic Request
        let refresh_token = request_message.refresh_token;

        //-- 2. Get & Validate  the Refresh Token Claim
        // Get the Token Secret from config and wrap it in a Secret to help limit leaks
        let token_secret = &self.config_ref().application.token_secret;
        let token_secret = Secret::new(token_secret.to_owned());

        // Using the Token Secret decode the token into a Token Claim
        // This also validates the token expiration, not before and Issuer
        let refresh_token_claim = domain::TokenClaim::from_token(
            &refresh_token,
            &token_secret,
        )
            .map_err(|_| {
                tracing::error!("Refresh Token is invalid!");
                BackendError::AuthenticationError("Authentication Failed!".to_string())
            })?;

        //-- 3. Get Refresh Token from database
        let database_record =
            database::RefreshTokens::from_token(&refresh_token, self.database_ref())
                .await?;

        // Revoke all Refresh Tokens associated with user_id
        let rows_affected = database_record
            .revoke_associated(self.database_ref())
            .await? as i64;

        // Build Tonic response message
        let response_message = LogoutResponse { rows_affected };

        // Send Response
        Ok(Response::new(response_message))
    }
}

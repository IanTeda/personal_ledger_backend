//-- ./src/database/sessions/insert.rs

//! Insert a Sessions into the database, returning a Result with a instance
//! ---

// #![allow(unused)] // For development only

use sqlx::{Pool, Postgres};

use crate::prelude::*;

use super::Sessions;

impl Sessions {
    /// Insert a Sessions into the database, returning sessions instance from the 
    /// database.
    ///
    /// # Parameters
    ///
    /// * `self` - A sessions instance
    /// * `database` - An Sqlx database connection pool
    /// ---
    #[tracing::instrument(
        name = "Insert a new Sessions into the database: ",
        skip(database)
    )]
    pub async fn insert(
        &self,
        database: &Pool<Postgres>,
    ) -> Result<Self, BackendError> {
        let database_record = sqlx::query_as!(
            Sessions,
            r#"
				INSERT INTO sessions (id, user_id, refresh_token, is_active, created_on)
				VALUES ($1, $2, $3, $4, $5) 
				RETURNING *
			"#,
            self.id,
            self.user_id,
            self.refresh_token.as_ref(),
            self.is_active,
            self.created_on
        )
        .fetch_one(database)
        .await?;

        tracing::debug!(
            "Sessions database records retrieved: {database_record:#?}"
        );

        Ok(database_record)
    }
}

//-- Unit Tests
#[cfg(test)]
pub mod tests {
    use sqlx::{Pool, Postgres};

    use crate::database;

    // Override with more flexible error
    pub type Result<T> = core::result::Result<T, Error>;
    pub type Error = Box<dyn std::error::Error>;

    // Test inserting into database
    #[sqlx::test]
    async fn create_database_record(database: Pool<Postgres>) -> Result<()> {
        //-- Setup and Fixtures (Arrange)
        // Generate random user for testing
        let random_user = database::Users::mock_data()?;

        // Insert user in the database
        random_user.insert(&database).await?;

        // Generate session
        let random_session =
            database::Sessions::mock_data(&random_user).await?;

        //-- Execute Function (Act)
        // Insert session into database
        let database_record = random_session.insert(&database).await?;

        //-- Checks (Assertions)
        assert_eq!(database_record, random_session);

        // -- Return
        Ok(())
    }
}

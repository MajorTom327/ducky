use std::fmt;

use ducky_core::{
    Catalog, Page, PageRequest, RelationDetails, RelationRef, ScriptReport, SessionInfo,
    TransactionMode,
};
use tokio_postgres::{CancelToken, Client};
use tokio_postgres_rustls::MakeRustlsConnect;

use crate::connect::parse_database_url;
use crate::error::{PgError, describe, query_error};
use crate::rows::column_value;
use crate::tls::make_tls_connector;

const SESSION_INFO_SQL: &str =
    "SELECT current_database()::text, current_user::text, current_setting('server_version')";

pub struct PgSession {
    browse: Client,
    script: Client,
    script_cancel_token: CancelToken,
    tls_connector: MakeRustlsConnect,
}

impl fmt::Debug for PgSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("PgSession").finish_non_exhaustive()
    }
}

impl PgSession {
    pub async fn connect(database_url: &str) -> Result<PgSession, PgError> {
        let target = parse_database_url(database_url)?;
        let tls_connector = make_tls_connector(target.verifies_certificates)?;
        let ((browse, browse_connection), (script, script_connection)) = tokio::try_join!(
            target.config.connect(tls_connector.clone()),
            target.config.connect(tls_connector.clone())
        )
        .map_err(|error| PgError::Connect(describe(&error)))?;
        tokio::spawn(async move {
            let _browse_connection_result = browse_connection.await;
        });
        tokio::spawn(async move {
            let _script_connection_result = script_connection.await;
        });
        let script_cancel_token = script.cancel_token();
        Ok(PgSession {
            browse,
            script,
            script_cancel_token,
            tls_connector,
        })
    }

    pub async fn info(&self) -> Result<SessionInfo, PgError> {
        let row = self
            .browse
            .query_one(SESSION_INFO_SQL, &[])
            .await
            .map_err(query_error)?;
        Ok(SessionInfo {
            database: column_value(&row, 0)?,
            user: column_value(&row, 1)?,
            server_version: column_value(&row, 2)?,
        })
    }

    pub async fn load_catalog(&self) -> Result<Catalog, PgError> {
        crate::catalog::load_catalog(&self.browse).await
    }

    pub async fn load_relation_details(
        &self,
        relation: &RelationRef,
    ) -> Result<RelationDetails, PgError> {
        crate::details::load_relation_details(&self.browse, relation).await
    }

    pub async fn load_page(&self, request: &PageRequest) -> Result<Page, PgError> {
        crate::browse::load_page(&self.browse, request).await
    }

    pub async fn run_script(&self, statements: &[String], mode: TransactionMode) -> ScriptReport {
        crate::script::run_script(&self.script, statements, mode).await
    }

    pub async fn cancel_script(&self) -> Result<(), PgError> {
        self.script_cancel_token
            .cancel_query(self.tls_connector.clone())
            .await
            .map_err(query_error)
    }
}

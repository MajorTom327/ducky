use std::sync::{Arc, Mutex, PoisonError};

use ducky_app::{Connection, Msg};
use ducky_core::{PgTools, Profile};
use ducky_postgres::PgSession;
use tokio::sync::mpsc::UnboundedSender;

pub(crate) const NOT_CONNECTED: &str = "Not connected";

#[derive(Debug, Clone)]
pub(crate) struct ActiveConnection {
    pub(crate) session: Arc<PgSession>,
    pub(crate) database_url: String,
    pub(crate) database: String,
    pub(crate) tools: PgTools,
}

pub(crate) type ActiveSlot = Arc<Mutex<Option<ActiveConnection>>>;

pub(crate) fn current_connection(active: &ActiveSlot) -> Option<ActiveConnection> {
    active
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

pub(crate) fn clear_connection(active: &ActiveSlot) {
    *active.lock().unwrap_or_else(PoisonError::into_inner) = None;
}

fn replace_connection(active: &ActiveSlot, connection: ActiveConnection) {
    *active.lock().unwrap_or_else(PoisonError::into_inner) = Some(connection);
}

pub(crate) fn spawn_connect(
    active: ActiveSlot,
    messages: UnboundedSender<Msg>,
    profile_index: usize,
    profile: Profile,
) {
    tokio::spawn(async move {
        let result = connect(&profile).await.map(|connection| {
            let answer = Connection {
                session: connection.session_info.clone(),
                tools: connection.active.tools.clone(),
            };
            replace_connection(&active, connection.active);
            answer
        });
        let _ = messages.send(Msg::Connected {
            profile_index,
            result,
        });
    });
}

struct EstablishedConnection {
    active: ActiveConnection,
    session_info: ducky_core::SessionInfo,
}

async fn connect(profile: &Profile) -> Result<EstablishedConnection, String> {
    let database_url = ducky_config::expand_env(&profile.url, &|name| std::env::var(name).ok())
        .map_err(|error| error.to_string())?;
    let session = PgSession::connect(&database_url)
        .await
        .map_err(|error| error.to_string())?;
    let session_info = session.info().await.map_err(|error| error.to_string())?;
    let tools = ducky_files::detect_tools(std::env::var_os("PATH").as_deref());
    Ok(EstablishedConnection {
        active: ActiveConnection {
            session: Arc::new(session),
            database_url,
            database: session_info.database.clone(),
            tools,
        },
        session_info,
    })
}

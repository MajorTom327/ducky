use std::time::Duration;

use ducky_app::{Msg, Notice, NoticeLevel};
use ducky_core::{PageRequest, RelationRef, ScriptFailure, ScriptReport, TransactionMode};
use tokio::sync::mpsc::UnboundedSender;

use crate::connection::{ActiveSlot, NOT_CONNECTED, current_connection};

pub(crate) fn spawn_load_catalog(active: &ActiveSlot, messages: UnboundedSender<Msg>) {
    let connection = current_connection(active);
    tokio::spawn(async move {
        let result = match connection {
            Some(connection) => connection
                .session
                .load_catalog()
                .await
                .map_err(|error| error.to_string()),
            None => Err(NOT_CONNECTED.to_owned()),
        };
        let _ = messages.send(Msg::CatalogLoaded(result));
    });
}

pub(crate) fn spawn_load_page(
    active: &ActiveSlot,
    messages: UnboundedSender<Msg>,
    request: PageRequest,
) {
    let connection = current_connection(active);
    tokio::spawn(async move {
        let result = match connection {
            Some(connection) => connection
                .session
                .load_page(&request)
                .await
                .map_err(|error| error.to_string()),
            None => Err(NOT_CONNECTED.to_owned()),
        };
        let _ = messages.send(Msg::PageLoaded { request, result });
    });
}

pub(crate) fn spawn_load_details(
    active: &ActiveSlot,
    messages: UnboundedSender<Msg>,
    relation: RelationRef,
) {
    let connection = current_connection(active);
    tokio::spawn(async move {
        let result = match connection {
            Some(connection) => connection
                .session
                .load_relation_details(&relation)
                .await
                .map_err(|error| error.to_string()),
            None => Err(NOT_CONNECTED.to_owned()),
        };
        let _ = messages.send(Msg::DetailsLoaded { relation, result });
    });
}

pub(crate) fn spawn_run_script(
    active: &ActiveSlot,
    messages: UnboundedSender<Msg>,
    sql: String,
    mode: TransactionMode,
) {
    let connection = current_connection(active);
    tokio::spawn(async move {
        let report = match connection {
            Some(connection) => {
                let statements = ducky_sql::split_statements(&sql);
                connection.session.run_script(&statements, mode).await
            }
            None => ScriptReport {
                mode,
                results: Vec::new(),
                failure: Some(ScriptFailure {
                    statement_index: 0,
                    sql,
                    message: NOT_CONNECTED.to_owned(),
                }),
                elapsed: Duration::ZERO,
            },
        };
        let _ = messages.send(Msg::ScriptFinished(report));
    });
}

pub(crate) fn spawn_cancel_script(active: &ActiveSlot, messages: UnboundedSender<Msg>) {
    let connection = current_connection(active);
    tokio::spawn(async move {
        let result = match connection {
            Some(connection) => connection
                .session
                .cancel_script()
                .await
                .map_err(|error| error.to_string()),
            None => Err(NOT_CONNECTED.to_owned()),
        };
        if let Err(message) = result {
            let _ = messages.send(Msg::Notified(Notice {
                level: NoticeLevel::Error,
                text: message,
            }));
        }
    });
}

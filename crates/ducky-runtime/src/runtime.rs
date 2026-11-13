use ducky_app::{Command, Msg, Notice, NoticeLevel};
use ducky_core::StoragePaths;
use ducky_files::{DumpStore, SqlStore};
use tokio::sync::mpsc::UnboundedSender;

use crate::clipboard::ClipboardSlot;
use crate::connection::{ActiveSlot, clear_connection, spawn_connect};
use crate::{database, files, jobs};

pub struct Runtime {
    messages: UnboundedSender<Msg>,
    sql_store: SqlStore,
    dump_store: DumpStore,
    active: ActiveSlot,
    clipboard: ClipboardSlot,
}

impl Runtime {
    pub fn new(storage: StoragePaths, messages: UnboundedSender<Msg>) -> Runtime {
        Runtime {
            messages,
            sql_store: SqlStore::new(storage.sql_dir),
            dump_store: DumpStore::new(storage.dump_dir),
            active: ActiveSlot::default(),
            clipboard: ClipboardSlot::default(),
        }
    }

    pub fn dispatch(&mut self, command: Command) {
        let messages = self.messages.clone();
        match command {
            Command::Connect {
                profile_index,
                profile,
            } => spawn_connect(self.active.clone(), messages, profile_index, profile),
            Command::Disconnect => clear_connection(&self.active),
            Command::LoadCatalog => database::spawn_load_catalog(&self.active, messages),
            Command::LoadPage(request) => {
                database::spawn_load_page(&self.active, messages, request)
            }
            Command::LoadDetails(relation) => {
                database::spawn_load_details(&self.active, messages, relation)
            }
            Command::RunScript { sql, mode } => {
                database::spawn_run_script(&self.active, messages, sql, mode)
            }
            Command::CancelScript => database::spawn_cancel_script(&self.active, messages),
            Command::ListSqlFiles => files::spawn_list_sql_files(self.sql_store.clone(), messages),
            Command::LoadFilePreview { name } => {
                files::spawn_load_preview(self.sql_store.clone(), messages, name)
            }
            Command::OpenFile {
                name,
                run_after_open,
            } => files::spawn_open_file(self.sql_store.clone(), messages, name, run_after_open),
            Command::SaveFile {
                name,
                content,
                overwrite,
            } => files::spawn_save_file(self.sql_store.clone(), messages, name, content, overwrite),
            Command::ListDumps => files::spawn_list_dumps(self.dump_store.clone(), messages),
            Command::StartDump { custom_file_name } => jobs::spawn_dump(
                &self.active,
                self.dump_store.clone(),
                messages,
                custom_file_name,
            ),
            Command::StartRestore { file_name, options } => jobs::spawn_restore(
                &self.active,
                self.dump_store.clone(),
                messages,
                file_name,
                options,
            ),
            Command::CopyToClipboard(text) => {
                if let Err(reason) = self.clipboard.copy(text) {
                    let _ = messages.send(Msg::Notified(Notice {
                        level: NoticeLevel::Error,
                        text: format!("Clipboard unavailable: {reason}"),
                    }));
                }
            }
        }
    }
}

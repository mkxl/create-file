use anyhow::{Context, Error as AnyhowError};
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use derive_more::From;
use mkutils::{Tracing, Utils};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::HashMap, io::Read};

type EntryMap = HashMap<Utf8PathBuf, Entry>;

#[derive(Deserialize, From, Serialize)]
struct DbRow {
    file: Utf8PathBuf,
}

#[derive(Deserialize)]
struct Entry {
    source: Utf8PathBuf,
}

#[derive(Parser)]
pub struct CliArgs {
    #[arg(long = "db-file")]
    database_filepath: Option<Utf8PathBuf>,

    #[arg(long = "entries-file")]
    entries_filepath: Option<Utf8PathBuf>,
}

impl CliArgs {
    const DEFAULT_DATABASE_FILENAME: &str = "create-file-db.json";
    const MISSING_HOME_DIRPATH_ERROR_MESSAGE: &str = "unable to identify home directory for default database filepath";
    const PARSE_ENTRY_MAP_ERROR_MESSAGE: &str = "unable to parse entries file";
    const READ_ENTRY_MAP_ERROR_MESSAGE: &str = "unable to read entries file";
    const READ_DATABASE_ERROR_MESSAGE: &str = "unable to read database file";
    const PARSE_DATABASE_ERROR_MESSAGE: &str = "unable to parse database file";
    const WRITE_DATABASE_ERROR_MESSAGE: &str = "unable to write database file";

    fn get_database_filepath(&self) -> Result<Cow<'_, Utf8Path>, AnyhowError> {
        if let Some(database_filepath) = self.database_filepath.as_deref() {
            database_filepath.to_cow_borrowed().ok()
        } else {
            Utf8PathBuf::home_dirpath()
                .context(Self::MISSING_HOME_DIRPATH_ERROR_MESSAGE)?
                .join_path(Self::DEFAULT_DATABASE_FILENAME)
                .into_cow_owned::<Utf8Path>()
                .ok()
        }
    }

    fn get_database_rows(database_filepath: &Utf8Path) -> Result<Vec<DbRow>, AnyhowError> {
        database_filepath
            .open()
            .context_path(Self::READ_DATABASE_ERROR_MESSAGE, database_filepath)?
            .buf_reader()
            .to_value_from_json_reader::<Vec<DbRow>>()
            .context_path(Self::PARSE_DATABASE_ERROR_MESSAGE, database_filepath)?
            .ok()
    }

    fn delete_database_files(database_filepath: &Utf8Path) {
        let Ok(database_rows) = Self::get_database_rows(database_filepath).log_if_error() else {
            return;
        };

        for db_row in database_rows {
            let io_res = db_row.file.remove_file();

            mkutils::trace!(
                level = io_res.level(),
                status = %io_res.status_display(),
                file = %db_row.file,
                message = "delete file"
            );
        }
    }

    fn get_entry_map_from_reader<R: Read>(reader: R) -> Result<EntryMap, AnyhowError> {
        reader
            .buf_reader()
            .to_value_from_json_reader::<EntryMap>()
            .context(Self::PARSE_ENTRY_MAP_ERROR_MESSAGE)
    }

    fn get_entry_map(&self) -> Result<HashMap<Utf8PathBuf, Entry>, AnyhowError> {
        if let Some(entries_filepath) = &self.entries_filepath {
            entries_filepath
                .open()
                .context_path(Self::READ_ENTRY_MAP_ERROR_MESSAGE, entries_filepath)?
                .pipe_into(Self::get_entry_map_from_reader)
        } else {
            std::io::stdin().lock().pipe_into(Self::get_entry_map_from_reader)
        }
    }

    fn copy_entry_source_to_dst_filepath(dst_filepath: &Utf8Path, entry: &Entry) -> Result<(), AnyhowError> {
        if let Some(dst_parent_dirpath) = dst_filepath.parent() {
            dst_parent_dirpath.create_dir_all()?;
        }

        entry.source.copy_to(dst_filepath)?;

        ().ok()
    }

    fn get_database_row((dst_filepath, entry): (Utf8PathBuf, Entry)) -> Option<DbRow> {
        let unit_res = Self::copy_entry_source_to_dst_filepath(&dst_filepath, &entry);

        mkutils::trace!(
            level = unit_res.level(),
            status = %unit_res.status_display(),
            src = %entry.source,
            dst = %dst_filepath,
            message = "process entry"
        );

        if unit_res.is_ok() {
            dst_filepath.convert::<DbRow>().some()
        } else {
            None
        }
    }

    fn write_database(database_rows: &[DbRow], database_filepath: &Utf8Path) -> Result<(), AnyhowError> {
        let database_file = database_filepath
            .create()
            .context_path(Self::WRITE_DATABASE_ERROR_MESSAGE, database_filepath)?;

        database_rows
            .write_as_json_to(database_file)
            .context(Self::WRITE_DATABASE_ERROR_MESSAGE)?
            .ok()
    }

    pub fn run(&self) {
        Tracing::default().init();

        let database_filepath_res = self.get_database_filepath().log_if_error();

        if let Ok(database_filepath) = &database_filepath_res {
            Self::delete_database_files(database_filepath);
        }

        let database_rows = if let Ok(entry_map) = self.get_entry_map().log_if_error() {
            entry_map.into_iter().filter_map(Self::get_database_row).collect()
        } else {
            Vec::new()
        };

        if let Ok(database_filepath) = &database_filepath_res {
            let unit_res = Self::write_database(&database_rows, database_filepath);

            mkutils::trace!(
                level = unit_res.level(),
                status = %unit_res.status_display(),
                database_filepath = %database_filepath.as_borrowed(),
                message = "write db file",
            );
        }
    }
}

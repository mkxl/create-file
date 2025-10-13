use anyhow::Context;
use anyhow::Error as AnyhowError;
use clap::Parser;
use derive_more::From;
use mkutils::Tracing;
use mkutils::Utils;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::path::Path;
use std::path::PathBuf;

#[derive(Deserialize)]
struct Entry {
    src: PathBuf,
    dst: PathBuf,
}

#[derive(Deserialize, From, Serialize)]
struct DbRow {
    file: PathBuf,
}

#[derive(Parser)]
pub struct App {
    #[arg(long = "db")]
    db_filepath: Option<PathBuf>,

    #[arg(long = "entries")]
    entries_filepath: Option<PathBuf>,
}

impl App {
    const DEFAULT_DB_FILENAME: &'static str = "create-file-db.json";

    #[mkutils::context("unable to get DB filepath")]
    fn db_filepath(&self) -> Result<Cow<Path>, AnyhowError> {
        if let Some(db_filepath) = self.db_filepath.as_deref() {
            return db_filepath.borrowed().ok::<AnyhowError>();
        }

        #[allow(deprecated)]
        let mut db_filepath = std::env::home_dir()
            .context("unable to identify for home directory for default DB filepath")?;

        db_filepath.push(Self::DEFAULT_DB_FILENAME);

        db_filepath.owned::<Path>().ok()
    }

    #[mkutils::context("unable to get DB rows")]
    fn db_rows(db_filepath: &Path) -> Result<Vec<DbRow>, AnyhowError> {
        db_filepath
            .open()
            .context_path("unable to open DB file", db_filepath)?
            .buf_reader()
            .json_from_reader::<Vec<DbRow>>()
            .context_path("unable to parse DB file", db_filepath)?
            .ok()
    }

    fn delete_db_files(db_filepath: &Path) {
        let Ok(old_db_rows) = Self::db_rows(db_filepath).log_if_error() else {
            return;
        };

        for db_row in old_db_rows {
            let status = db_row.file.remove_file().into_status();

            mkutils::trace!(level = status.level(), %status, file = %db_row.file.display(), "delete file");
        }
    }

    #[mkutils::context("unable to get entries")]
    fn entries(&self) -> Result<Vec<Entry>, AnyhowError> {
        if let Some(entries_filepath) = &self.entries_filepath {
            entries_filepath
                .open()
                .context_path("unable to open entries file", entries_filepath)?
                .buf_reader()
                .json_from_reader::<Vec<Entry>>()
        } else {
            std::io::stdin()
                .lock()
                .buf_reader()
                .json_from_reader::<Vec<Entry>>()
        }
        .context("unable to parse entries file")?
        .ok::<AnyhowError>()
    }

    fn process_entry(new_db_rows: &mut Vec<DbRow>, entry: &Entry) -> Result<(), AnyhowError> {
        if let Some(dst_parent_dirpath) = entry.dst.parent() {
            dst_parent_dirpath.create_dir_all()?;
        }

        std::fs::copy(&entry.src, &entry.dst)?;
        new_db_rows.push(entry.dst.clone().into());

        ().ok()
    }

    fn process_entries(new_db_rows: &mut Vec<DbRow>, entries: Vec<Entry>) {
        for entry in entries {
            let status = Self::process_entry(new_db_rows, &entry).into_status();

            mkutils::trace!(
                level = status.level(),
                %status,
                src = %entry.src.display(),
                dst = %entry.dst.display(),
                "process entry"
            );
        }
    }

    fn write_db(db_rows: &[DbRow], db_filepath: &Path) -> Result<(), AnyhowError> {
        let db_file = db_filepath
            .create()
            .context_path("unable to create new DB file", db_filepath)?;

        db_rows
            .json_to_writer(db_file)
            .context("unable to write to new DB file")?
            .ok()
    }

    pub fn run(&self) -> Result<(), AnyhowError> {
        Tracing::default().init();

        let db_filepath_res = self.db_filepath().log_if_error();
        let mut new_db_rows = Vec::new();

        if let Ok(db_filepath) = &db_filepath_res {
            Self::delete_db_files(db_filepath);
        }

        if let Ok(entries) = self.entries().log_if_error() {
            Self::process_entries(&mut new_db_rows, entries);
        }

        if let Ok(db_filepath) = &db_filepath_res {
            let status = Self::write_db(&new_db_rows, db_filepath).into_status();

            mkutils::trace!(level = status.level(), %status, db_filepath = %db_filepath.display(), "write db file");
        }

        ().ok()
    }
}

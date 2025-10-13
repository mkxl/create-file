mod app;

use crate::app::App;
use anyhow::Error as AnyhowError;
use clap::Parser;

fn main() -> Result<(), AnyhowError> {
    App::parse().run()
}

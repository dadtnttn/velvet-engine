use std::path::PathBuf;

use anyhow::Result;
use velvet_vs3_app::{run_package, AppOptions};

fn main() -> Result<()> {
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
    run_package(package, AppOptions::from_env_args()?)
}

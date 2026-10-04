use std::path::Path;

use eyre::Result;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;

pub fn create_session<P: AsRef<Path>>(path: P) -> Result<Session> {
    let session = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|error| eyre::eyre!("{error}"))?
        .with_intra_threads(1)
        .map_err(|error| eyre::eyre!("{error}"))?
        .with_inter_threads(1)
        .map_err(|error| eyre::eyre!("{error}"))?
        .commit_from_file(path.as_ref())?;
    Ok(session)
}

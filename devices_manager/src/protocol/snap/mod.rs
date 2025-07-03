mod gateway;
mod payload;

pub use gateway::*;
pub use payload::DownloadData;
pub use payload::UpData;

#[derive(Debug)]
pub enum CustomError {
    Key,
    Format(String),
    MIC,
}

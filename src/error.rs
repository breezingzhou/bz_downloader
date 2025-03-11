#[derive(Debug, thiserror::Error)]
pub enum BzError {
  #[error("Init Error: {reason}")]
  InitError { reason: &'static str },
}

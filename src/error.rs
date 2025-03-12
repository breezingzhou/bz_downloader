use crate::bz_task::{BzTaskControl, BzTaskId, BzTaskMessage, BzTaskStatus};


#[derive(Debug, thiserror::Error)]
pub enum BzError {
  #[error("Init Error: {reason}")]
  InitError { reason: &'static str },
  #[error(" Task NotFound task_id: {0}")]
  TaskNotFound(BzTaskId),
  #[error(" Runtime NotFound task_id: {0}")]
  RuntimeNotFound(BzTaskId),
  #[error("Send BzTaskControl Error: {0}")]
  MpscBzTaskControlError(#[from] tokio::sync::mpsc::error::TrySendError<BzTaskControl>),
  #[error("Task Status Error! current_status: {0} current_action: {1}")]
  TaskStatusError(BzTaskStatus, BzTaskMessage),
}


pub type BzResult<T> = std::result::Result<T, BzError>;
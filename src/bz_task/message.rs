use crate::{
  app_state::AppState, bz_downloader::Message, bz_task::{self, BzTask, BzTaskRuntimeInfo, BzTaskStatus}, error::{BzError, BzResult}
};
use iced::Task as Command;

use super::{BzTaskId, BzTaskInfo};

// 前端交互发送的消息 在iced update中处理
#[derive(Debug, Clone)]
pub enum BzTaskMessage {
  AddTask(BzTaskInfo),
  TryStartTask(BzTaskId),
  StartTask(BzTaskId),
  TryStopTask(BzTaskId),
  StopTask(BzTaskId),
  RemoveTask(BzTaskId),
  FinishTask(BzTaskId),
  FailTask(BzTaskId),
}

impl std::fmt::Display for BzTaskMessage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      BzTaskMessage::AddTask(task_info) => {
        write!(f, "AddTask: {:?}", task_info)
      }
      BzTaskMessage::TryStartTask(task_id) => {
        write!(f, "TryStartTask: {:?}", task_id)
      }
      BzTaskMessage::StartTask(task_id) => {
        write!(f, "StartTask: {:?}", task_id)
      }
      BzTaskMessage::TryStopTask(task_id) => {
        write!(f, "TryStopTask: {:?}", task_id)
      }
      BzTaskMessage::StopTask(task_id) => write!(f, "StopTask: {:?}", task_id),
      BzTaskMessage::RemoveTask(task_id) => {
        write!(f, "RemoveTask: {:?}", task_id)
      }
      BzTaskMessage::FinishTask(task_id) => {
        write!(f, "FinishTask: {:?}", task_id)
      }
      BzTaskMessage::FailTask(task_id) => {
        write!(f, "FailTask: {:?}", task_id)
      }
    }
  }
}

pub fn get_task_from_btreemap(
  app_state: &mut AppState, task_id: BzTaskId,
) -> BzResult<&mut BzTask> {
  let task = app_state
    .tasks
    .get_mut(&task_id)
    .ok_or(BzError::TaskNotFound(task_id))?;
  Ok(task)
}

pub fn get_runtime_from_task(
  task: &mut BzTask,
) -> BzResult<&BzTaskRuntimeInfo> {
  let runtime = task
    .runtime
    .as_ref()
    .ok_or(BzError::RuntimeNotFound(task.id))?;
  Ok(runtime)
}

pub fn get_runtime_from_btreemap(
  app_state: &mut AppState, task_id: BzTaskId,
) -> BzResult<&BzTaskRuntimeInfo> {
  let task = get_task_from_btreemap(app_state, task_id)?;
  let runtime = task
    .runtime
    .as_ref()
    .ok_or(BzError::RuntimeNotFound(task_id))?;
  Ok(runtime)
}

// 处理前端发送的消息
pub fn deal_bztask_message(
  app_state: &mut AppState, task_message: BzTaskMessage,
) -> BzResult<Command<Message>> {
  let cmd = match task_message {
    BzTaskMessage::AddTask(task_info) => {
      log::debug!("[BzTaskMessage::AddTask] : {:?}", task_info);
      let task = BzTask::from_info(task_info);
      let task_id = task.id;
      app_state.tasks.insert(task.id, task);
      Command::done(Message::BzTask(BzTaskMessage::TryStartTask(task_id)))
    }
    BzTaskMessage::TryStartTask(task_id) => {
      log::debug!("[BzTaskMessage::TryStartTask]: {:?}", task_id);
      let feedback_sender = app_state.feedback_sender.clone();
      let task = assert_task_status(
        app_state,
        task_id,
        &vec![BzTaskStatus::Queued, BzTaskStatus::Stopped],
        &task_message,
      )?;
      let (control_sender, join_handle) =
        bz_task::run_task(task.id, task.info.clone(), feedback_sender);
      task.runtime = Some(BzTaskRuntimeInfo {
        sender: control_sender,
        join_handle,
      });
      Command::none()
    }
    BzTaskMessage::StartTask(task_id) => {
      log::debug!("[BzTaskMessage::StartTask]: {:?}", task_id);
      let task = assert_task_status(
        app_state,
        task_id,
        &vec![BzTaskStatus::Queued, BzTaskStatus::Stopped],
        &task_message,
      )?;
      task.info.status = BzTaskStatus::Running;
      Command::none()
    }
    BzTaskMessage::TryStopTask(task_id) => {
      log::debug!("[BzTaskMessage::StopTask]: {:?}", task_id);
      let task = assert_task_status(
        app_state,
        task_id,
        &vec![BzTaskStatus::Running],
        &task_message,
      )?;
      let runtime = get_runtime_from_task(task)?;
      let _ = runtime.sender.try_send(bz_task::BzTaskControl::Stop)?;
      Command::none()
    }
    BzTaskMessage::StopTask(task_id) => {
      log::debug!("[BzTaskMessage::StopTask]: {:?}", task_id);
      let task = assert_task_status(
        app_state,
        task_id,
        &vec![BzTaskStatus::Running],
        &task_message,
      )?;
      // TODO 这里需要考虑joinhandle的问题
      task.info.status = bz_task::BzTaskStatus::Stopped;
      Command::none()
    }
    BzTaskMessage::RemoveTask(task_id) => {
      log::debug!("[BzTaskMessage::RemoveTask]: {:?}", task_id);
      let _task = assert_task_status(
        app_state,
        task_id,
        &vec![BzTaskStatus::Stopped],
        &task_message,
      )?;
      app_state.tasks.remove(&task_id);
      Command::none()
    }
    BzTaskMessage::FinishTask(task_id) => {
      log::debug!("[BzTaskMessage::FinishTask]: {:?}", task_id);
      let task = assert_task_status(
        app_state,
        task_id,
        &vec![BzTaskStatus::Running],
        &task_message,
      )?;
      task.info.status = bz_task::BzTaskStatus::Completed;
      task.extra.progress = 1.0;
      Command::none()
    }
    BzTaskMessage::FailTask(task_id) => {
      log::debug!("[BzTaskMessage::FailTask]: {:?}", task_id);
      let task = assert_task_status(
        app_state,
        task_id,
        &vec![BzTaskStatus::Running],
        &task_message,
      )?;
      task.info.status = bz_task::BzTaskStatus::Failed;
      Command::none()
    }
  };
  Ok(cmd)
}

pub fn assert_task_status<'a>(
  app_state: &'a mut AppState, task_id: BzTaskId,
  status_list: &Vec<BzTaskStatus>, task_message: &BzTaskMessage,
) -> BzResult<&'a mut BzTask> {
  let task = get_task_from_btreemap(app_state, task_id)?;
  if !status_list.contains(&task.info.status) {
    return Err(BzError::TaskStatusError(
      task.info.status,
      task_message.clone(),
    ));
  }
  Ok(task)
}

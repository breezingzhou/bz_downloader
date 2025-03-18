use iced::{
  futures::{SinkExt, Stream},
  stream,
};
use tokio::{sync::mpsc, task::JoinHandle};

use crate::{
  bz_downloader::Message,
  bz_task::{BzTaskControl, BzTaskFeedBack, BzTaskInfo},
  m3u8::M3u8Task,
  zfs::ZfsTask,
};

use super::{
  BzTaskControlFeedBackMessage, BzTaskId, BzTaskInfoFeedBackMessage,
  BzTaskMessage, BzTaskType, info::BzTaskControlFeedBack,
};

// Task Progress

pub trait TaskProgress {
  type Message;
  fn load(&mut self);
  fn dump(&self);
  fn _update(&mut self, message: Self::Message);
  fn update(&mut self, message: Self::Message) {
    self._update(message);
    self.dump();
  }
  fn rate(&self) -> f32;
}

// 后端所代表的任务
pub trait Task {
  fn new_task(task_info: BzTaskInfo) -> Self;
  async fn prepare(&mut self);
  async fn start(
    &mut self, task_id: BzTaskId,
    control_receiver: mpsc::Receiver<BzTaskControl>,
    feedback_sender: mpsc::Sender<BzTaskFeedBack>,
  ) -> bool;
  async fn finish(&mut self);
}

pub async fn run_task_impl<T: Task>(
  task_id: BzTaskId, task_info: BzTaskInfo,
  control_receiver: mpsc::Receiver<BzTaskControl>,
  feedback_sender: mpsc::Sender<BzTaskFeedBack>,
) {
  let mut task: T = T::new_task(task_info);
  task.prepare().await;
  let _ = feedback_sender
    .send(BzTaskFeedBack::TaskConrol(BzTaskControlFeedBackMessage {
      task_id,
      control: BzTaskControlFeedBack::Started,
    }))
    .await;
  let is_finished = task
    .start(task_id, control_receiver, feedback_sender.clone())
    .await;
  if is_finished {
    task.finish().await;
    let _ = feedback_sender
      .send(BzTaskFeedBack::TaskConrol(BzTaskControlFeedBackMessage {
        task_id,
        control: BzTaskControlFeedBack::Finished,
      }))
      .await;
  }
}

// 创建两个channel 一个用于发送控制信息 一个用于接受进度信息
pub fn run_task(
  task_id: BzTaskId, task_info: BzTaskInfo,
  feedback_sender: mpsc::Sender<BzTaskFeedBack>,
) -> (mpsc::Sender<BzTaskControl>, JoinHandle<()>) {
  let (control_sender, control_receiver) = mpsc::channel::<BzTaskControl>(100);
  let handle = tokio::spawn(async move {
    match task_info.kind {
      BzTaskType::M3u8 => {
        run_task_impl::<M3u8Task>(
          task_id,
          task_info,
          control_receiver,
          feedback_sender,
        )
        .await
      }
      BzTaskType::Zfs => {
        run_task_impl::<ZfsTask>(
          task_id,
          task_info,
          control_receiver,
          feedback_sender,
        )
        .await
      }
    };
  });
  return (control_sender, handle);
}

// 供iced subscription使用 用于接受任务下载时候反馈的信息
pub fn feed_back_subscription() -> impl Stream<Item = Message> {
  stream::channel(100, |mut output| async move {
    let (sender, mut receiver) = mpsc::channel::<BzTaskFeedBack>(100);
    let _ = output.send(Message::FeedbackChannelCreated(sender)).await;
    loop {
      if let Some(message) = receiver.recv().await {
        match message {
          BzTaskFeedBack::TaskConrol(control_message) => {
            let task_id = control_message.task_id;
            let message = match control_message.control {
              BzTaskControlFeedBack::Started => {
                log::debug!(
                  "[subscription] Task Started: {:?}",
                  control_message.task_id
                );
                Message::BzTask(BzTaskMessage::StartTask(task_id))
              }
              BzTaskControlFeedBack::Stoped => {
                log::debug!(
                  "[subscription] Task Stoped: {:?}",
                  control_message.task_id
                );
                Message::BzTask(BzTaskMessage::StopTask(task_id))
              }
              BzTaskControlFeedBack::Finished => {
                log::debug!(
                  "[subscription] Task Finished: {:?}",
                  control_message.task_id
                );
                Message::BzTask(BzTaskMessage::FinishTask(task_id))
              }
              BzTaskControlFeedBack::Failed => {
                log::debug!(
                  "[subscription] Task Failed: {:?}",
                  control_message.task_id
                );
                Message::BzTask(BzTaskMessage::FailTask(task_id))
              }
            };

            let _ = output.send(message).await;
          }
          BzTaskFeedBack::TaskInfo(info_message) => {
            let message =
              Message::TaskInfoFeedBack(BzTaskInfoFeedBackMessage {
                task_id: info_message.task_id,
                progress: info_message.progress,
              });
            let _ = output.send(message).await;
          }
        }
      }
    }
  })
}

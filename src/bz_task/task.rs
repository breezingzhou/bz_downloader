use iced::{
  futures::{SinkExt, Stream},
  stream,
};
use tokio::sync::mpsc;

use crate::{
  bz_task::{BzTaskControl, BzTaskFeedBack, BzTaskInfo},
  m3u8::M3u8Task,
  zfs::ZfsTask,
};

use super::BzTaskId;

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

pub trait Task {
  type Progress;
  async fn prepare(&mut self);
  async fn start(
    &mut self, task_id: BzTaskId,
    control_receiver: mpsc::Receiver<BzTaskControl>,
    feedback_sender: mpsc::Sender<BzTaskFeedBack>,
  );
}

#[derive(Debug, Clone)]
pub enum BzTaskMessage {
  AddTask(BzTaskInfo),
  PauseTask(usize),
  RestartTask(usize),
  RemoveTask(usize),
}

// 创建两个channel 一个用于发送控制信息 一个用于接受进度信息
pub fn run_task(
  task_id: BzTaskId, task_info: BzTaskInfo,
  feedback_sender: mpsc::Sender<BzTaskFeedBack>,
) -> mpsc::Sender<BzTaskControl> {
  let (control_sender, control_receiver) = mpsc::channel::<BzTaskControl>(100);
  tokio::spawn(async move {
    let mut task = ZfsTask::new(task_info);
    task.prepare().await;
    task.start(task_id, control_receiver, feedback_sender).await;
  });
  return control_sender;
}

// 供iced subscription使用 用于接受任务下载时候反馈的信息
pub fn feed_back_subscription() -> impl Stream<Item = crate::Message> {
  stream::channel(100, |mut output| async move {
    let (sender, mut receiver) = mpsc::channel::<BzTaskFeedBack>(100);
    let _ = output
      .send(crate::Message::FeedbackChannelCreated(sender))
      .await;
    loop {
      if let Some(message) = receiver.recv().await {
        let _ = output.send(crate::Message::TaskFeedBack(message)).await;
      }
    }
  })
}

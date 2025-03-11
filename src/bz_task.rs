use std::path::Path;

use iced::{
  futures::{SinkExt, Stream},
  stream,
};
use reqwest::Url;
use std::path::PathBuf;
use tokio::sync::mpsc;

use crate::{m3u8::M3u8Task, zfs::ZfsTask};

#[derive(Debug, Clone, Copy)]
pub enum TaskStatus {
  Queued,
  Downloading(f32),
  Paused,
  Completed,
  Failed,
}

pub enum TaskInnerStatus {
  Started,
  Paused,
  Stopped,
  Failed,
}

#[derive(Debug, Clone)]
pub struct BzTaskInfo {
  pub src: Url,
  pub dest: PathBuf, // 下载目录
  pub temp: PathBuf, // 临时文件
  // TODO status 或者 后续的下载速度 之类的信息  应该是不会被dump的  后续抽象出来？
  pub status: TaskStatus,
}

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

#[derive(Debug, Clone)]
pub enum Control {
  Pause,
  Restart,
  Stop,
}

#[derive(Debug, Clone)]
pub struct BzTaskControlMessage {
  pub id: usize,
  pub control: Control,
}

#[derive(Debug, Clone)]
pub struct BzTaskFeedBackMessage {
  pub id: usize,
  pub progress: f32,
}

pub trait Task {
  type Progress;
  async fn prepare(&mut self);
  async fn start(&mut self, control_receiver: mpsc::Receiver<BzTaskControlMessage>, feedback_sender: mpsc::Sender<BzTaskFeedBackMessage>);
}

// 这里是想要抽象一个下载任务 可以支持不同的下载任务 比如m3u8 torrent?等
// 先跟m3u8的任务混一下

#[derive(Debug, Clone)]
pub enum BzTaskMessage {
  AddTask(String),
  PauseTask(usize),
  RestartTask(usize),
  RemoveTask(usize),
}

// 创建两个channel 一个用于发送控制信息 一个用于接受进度信息
pub fn run_task(
  task_info: BzTaskInfo,
  feedback_sender: mpsc::Sender<BzTaskFeedBackMessage>,
) -> mpsc::Sender<BzTaskControlMessage> {
  let (control_sender, control_receiver) =
    mpsc::channel::<BzTaskControlMessage>(100);
  tokio::spawn(async move {
    let mut task = ZfsTask::new(task_info);
    task.prepare().await;
    task.start(control_receiver, feedback_sender).await;
  });
  return control_sender;
}

// 供iced subscription使用 用于接受任务下载时候反馈的信息
pub fn feed_back_subscription() -> impl Stream<Item = crate::Message> {
  stream::channel(100, |mut output| async move {
    let (sender, mut receiver) =
      mpsc::channel::<BzTaskFeedBackMessage>(100);
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

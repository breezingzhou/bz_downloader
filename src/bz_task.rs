use std::path::Path;

use reqwest::Url;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub enum TaskStatus {
  Queued,
  Downloading(f32),
  Paused,
  Completed,
  Failed,
}

#[derive(Debug, Clone)]
pub struct BzTaskInfo {
  pub src: Url,
  pub dest: PathBuf, // 下载目录
  pub temp: PathBuf, // 临时文件
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

pub enum BzTaskControlMessage {
  Stop
}

pub trait Task {
  type Progress;
  async fn prepare(&mut self);
  async fn start(&mut self);
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

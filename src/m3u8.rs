use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::{collections::HashSet, time::Duration};

use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::bz_task::{
  BzTaskControl, BzTaskFeedBack, BzTaskId, BzTaskInfo, Control, TaskInnerStatus
};
use crate::bz_task::{Task, TaskProgress};

pub struct M3u8TaskProgress {
  pub save_file: PathBuf,
  pub downloaded: HashSet<String>,
  pub todos: Vec<String>,
  pub total: usize,
}

impl M3u8TaskProgress {
  pub fn new<P: AsRef<Path>>(temp_dir: P) -> Self {
    Self {
      save_file: temp_dir.as_ref().join("process.json"),
      downloaded: HashSet::new(),
      todos: Vec::new(),
      total: 0,
    }
  }

  pub fn init_tasks(&mut self, uris: &Vec<String>) {
    self.total = uris.len() as usize;
    for uri in uris {
      if !self.downloaded.contains(uri) {
        self.todos.push(uri.clone());
      }
    }
  }
}

pub enum M3u8TaskProgressMessage {
  Add(String),
  Remove(String),
}

impl TaskProgress for M3u8TaskProgress {
  type Message = M3u8TaskProgressMessage;
  fn load(&mut self) {
    let file = std::fs::File::open(&self.save_file);
    match file {
      Ok(f) => {
        let reader = std::io::BufReader::new(f);
        self.downloaded = serde_json::from_reader(reader).unwrap();
      }
      Err(_) => {
        log::warn!("no progress file found");
      }
    }
  }

  fn dump(&self) {
    let file = std::fs::File::create(&self.save_file);
    match file {
      Ok(f) => {
        let writer = std::io::BufWriter::new(f);
        serde_json::to_writer(writer, &self.downloaded).unwrap();
      }
      Err(_) => {
        log::error!("failed to create progress file");
      }
    }
  }

  fn _update(&mut self, message: Self::Message) {
    match message {
      M3u8TaskProgressMessage::Add(url) => {
        self.downloaded.insert(url);
      }
      M3u8TaskProgressMessage::Remove(url) => {
        self.downloaded.remove(&url);
      }
    }
  }

  fn rate(&self) -> f32 {
    return self.downloaded.len() as f32 / self.total as f32;
  }
}

pub struct M3u8Task {
  task_info: BzTaskInfo,
  porgress: M3u8TaskProgress,
  uris: Vec<String>,
}

impl M3u8Task {
  pub fn new(task_info: &BzTaskInfo) -> Self {
    Self {
      task_info: task_info.clone(),
      porgress: M3u8TaskProgress::new(&task_info.cache),
      uris: Vec::new(),
    }
  }

  // 获取索引文件
  // 如果本地有索引文件则返回本地索引文件
  // 否则下载并且返回
  async fn get_m3u8_index(&self) -> Vec<u8> {
    let index_file = PathBuf::from(&self.task_info.cache).join("index.m3u8");
    if index_file.exists() {
      let content = std::fs::read(index_file).unwrap();
      return content;
    } else {
      let content = reqwest::get(self.task_info.src.clone())
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
      std::fs::write(&index_file, &content).unwrap();
      return content.into();
    }
  }

  // 解析索引文件 获取ts文件列表
  pub async fn get_ts_file_list(&self) -> Vec<String> {
    let index_content = self.get_m3u8_index().await;
    let m3u8 = m3u8_rs::parse_media_playlist(&index_content).unwrap().1;
    let uris = m3u8
      .segments
      .into_iter()
      .map(|segment| segment.uri.clone())
      .collect::<Vec<String>>();
    uris
  }
}

impl Task for M3u8Task {
  type Progress = M3u8TaskProgress;

  async fn prepare(&mut self) {
    // 下载 m3u8 url
    // 解析 m3u8 获取需要下载哪些ts文件
    // 检查本地已经下载了那些文件
    // 设置后续需要下载的文件
    let ts_files = self.get_ts_file_list().await;
    self.porgress.load();
    self.porgress.init_tasks(&ts_files);
    self.uris = ts_files;
  }

  async fn start(
    &mut self, task_id: BzTaskId,
    mut control_receiver: tokio::sync::mpsc::Receiver<BzTaskControl>,
    feedback_sender: tokio::sync::mpsc::Sender<BzTaskFeedBack>,
  ) {
    // 下载ts文件
    // 更新下载进度
    let client = reqwest::Client::new();
    let mut status = TaskInnerStatus::Started;
    loop {
      if self.porgress.todos.is_empty() {
        break;
      }
      if let Ok(control_message) = control_receiver.try_recv() {
        status = match control_message.control {
          Control::Pause => {
            log::info!("task paused");
            TaskInnerStatus::Paused
          }
          Control::Restart => {
            log::info!("task restarted");
            TaskInnerStatus::Paused
          }
          Control::Stop => {
            log::info!("task stopped");
            TaskInnerStatus::Stopped
          }
        }
      }

      match status {
        TaskInnerStatus::Paused => {
          tokio::time::sleep(Duration::from_secs(1)).await;
          continue;
        }
        TaskInnerStatus::Stopped => {
          break;
        }
        _ => {}
      }

      let uri = self.porgress.todos.pop().unwrap();
      let file_path = self.task_info.cache.clone().join(&uri);
      let url = self.task_info.src.clone().join(&uri).unwrap();
      let content =
        client.get(url).send().await.unwrap().bytes().await.unwrap();
      // 这里可能有问题  创建了完文件就gg了  文件内容没有写入
      let mut file = fs::File::create(file_path).await.unwrap();
      file.write(&content).await.unwrap();
      self.porgress.update(M3u8TaskProgressMessage::Add(uri));
      let _ = feedback_sender
        .send(BzTaskFeedBack {
          task_id,
          progress: self.porgress.rate(),
        })
        .await;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::bz_task::{BzTaskId, BzTaskStatus, BzTaskType};
  use tokio;

  #[tokio::test]
  async fn test_m3u8_task() {
    env_logger::init();
    let task_info = BzTaskInfo {
      src: reqwest::Url::parse("https://svipsvip.ffzy-online5.com/20250118/37333_517b17a8/2000k/hls/mixed.m3u8").unwrap(),
      dest: PathBuf::from("./tmp"),
      cache: PathBuf::from("./tmp"),
      kind: BzTaskType::M3u8,
      status: BzTaskStatus::Queued,
    };
    let mut task = M3u8Task::new(&task_info);
  }
}

use std::time::Duration;

use crate::bz_task::{
  BzTaskControl, BzTaskFeedBack, BzTaskId, BzTaskInfo,
  BzTaskInfoFeedBackMessage, Task, TaskProgress,
};

pub struct ZfsTaskProgress {
  pub downloaded: Vec<String>,

  pub todos: Vec<String>,
  pub total: usize,
}

pub enum ZfsTaskProgressMessage {
  Add(String),
  Remove(String),
}

impl TaskProgress for ZfsTaskProgress {
  type Message = ZfsTaskProgressMessage;

  fn load(&mut self) {
    ()
  }

  fn dump(&self) {
    ()
  }

  fn _update(&mut self, message: Self::Message) {
    match message {
      ZfsTaskProgressMessage::Add(url) => {
        self.downloaded.push(url);
      }
      ZfsTaskProgressMessage::Remove(url) => {}
    }
  }

  fn rate(&self) -> f32 {
    self.downloaded.len() as f32 / self.total as f32
  }
}

pub struct ZfsTask {
  task_info: BzTaskInfo,
  porgress: ZfsTaskProgress,
  uris: Vec<String>,
}

impl ZfsTask {
  pub fn new(task_info: BzTaskInfo) -> Self {
    Self {
      task_info,
      porgress: ZfsTaskProgress {
        downloaded: vec![],
        todos: vec![],
        total: 10,
      },
      uris: vec![],
    }
  }
}

impl Task for ZfsTask {
  fn new_task(task_info: BzTaskInfo) -> Self {
    Self::new(task_info)
  }

  async fn prepare(&mut self) {}

  async fn start(
    &mut self, task_id: BzTaskId,
    _control_receiver: tokio::sync::mpsc::Receiver<BzTaskControl>,
    feedback_sender: tokio::sync::mpsc::Sender<BzTaskFeedBack>,
  ) -> bool {
    let mut i = 0;
    loop {
      if i == 10 {
        return true;
      }

      i = i + 1;
      tokio::time::sleep(Duration::from_secs(2)).await;
      let _ = feedback_sender
        .send(BzTaskFeedBack::TaskInfo(BzTaskInfoFeedBackMessage {
          task_id: task_id.clone(),
          progress: (i as f32 / 10.0),
        }))
        .await;
    }
  }

  async fn finish(&mut self) {
    todo!()
  }
}

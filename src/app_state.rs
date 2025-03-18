use crate::bz_task::{
  BzTask, BzTaskExtraInfo, BzTaskFeedBack, BzTaskId, BzTaskInfo,
};
use directories::ProjectDirs;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct AppPreState {
  pub tray_state: crate::tray::TrayState,
  pub task_infos: Option<Vec<BzTaskInfo>>,
  pub feedback_sender: Option<tokio::sync::mpsc::Sender<BzTaskFeedBack>>,
}

impl AppPreState {
  pub fn is_ready(&self) -> bool {
    self.task_infos.is_some() && self.feedback_sender.is_some()
  }
}


pub struct AppState {
  pub tray_state: crate::tray::TrayState,
  pub tasks: BTreeMap<BzTaskId, BzTask>,
  pub feedback_sender: tokio::sync::mpsc::Sender<BzTaskFeedBack>,
}

impl From<AppPreState> for AppState {
  fn from(app_pre_state: AppPreState) -> Self {
    let task_infos = app_pre_state.task_infos.unwrap();
    let tasks = task_infos
      .iter()
      .map(|task_info| {
        let id = BzTaskId::unique();
        let task = BzTask {
          id: id.clone(),
          info: task_info.clone(),
          extra: BzTaskExtraInfo::default(),
          runtime: None,
        };
        (id, task)
      })
      .collect();
    Self {
      tray_state: app_pre_state.tray_state,
      tasks: tasks,
      feedback_sender: app_pre_state.feedback_sender.unwrap(),
    }
  }
}


#[allow(non_snake_case)]
pub fn AppDir() -> ProjectDirs {
  let app_dir = ProjectDirs::from("com", "breezing", "bz_downloader").unwrap();
  return app_dir;
}

pub fn init_dirs() {
  let app_dir = AppDir();
  let cache_dir = app_dir.cache_dir();
  let data_local_dir = app_dir.data_local_dir();
  log::debug!("cache_dir: {:?}", cache_dir);
  log::debug!("data_local_dir: {:?}", data_local_dir);
  if !cache_dir.exists() {
    log::info!("cache_dir not exists");
    log::info!("create cache_dir: {}", cache_dir.display());
    std::fs::create_dir_all(cache_dir).unwrap();
  }
  if !data_local_dir.exists() {
    log::info!("data_dir not exists");
    log::info!("create data_dir: {}", data_local_dir.display());
    std::fs::create_dir_all(data_local_dir).unwrap();
  }
}

pub async fn load_data() -> Vec<BzTaskInfo> {
  let task_list = AppDir().data_local_dir().join("task_list.json");
  if !task_list.exists() {
    return Vec::new();
  }
  let task_infos: Vec<BzTaskInfo> =
    serde_json::from_reader(std::fs::File::open(task_list).unwrap()).unwrap();
  task_infos
}

pub async fn save_data(task_infos: Vec<BzTaskInfo>) {
  let task_list = AppDir().data_local_dir().join("task_list.json");
  serde_json::to_writer_pretty(
    std::fs::File::create(task_list).unwrap(),
    &task_infos,
  )
  .unwrap();
}

#[cfg(test)]
mod test {
  use directories::ProjectDirs;

  #[test]
  fn test_dirs() {
    let app_dir =
      ProjectDirs::from("com", "breezing", "bz_downloader").unwrap();
    println!("{:?}", app_dir);
  }
}

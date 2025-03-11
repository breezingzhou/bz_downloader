mod bz_task;
mod error;
mod m3u8;
mod tray;
mod view;
mod zfs;

use bz_task::{
  BzTask, BzTaskExtraInfo, BzTaskId, BzTaskMessage, BzTaskRuntimeInfo,
};
use bz_task::{BzTaskFeedBack, BzTaskInfo, BzTaskStatus, BzTaskType};
use error::BzError;
use iced::{
  Element, Font, Subscription, Task as Command, keyboard,
  widget::{Text, column, horizontal_rule},
  window::{self, Mode},
};
use std::collections::BTreeMap;
use std::default;
use std::time::Duration;
use tray::BzMenuType;
use tray_icon::{TrayIcon, menu::MenuEvent};

pub fn main() -> iced::Result {
  env_logger::Builder::new()
    .filter_module("bz_downloader", log::LevelFilter::Debug)
    .init();

  iced::application("BzDownloader", BzDownloader::update, BzDownloader::view)
    .subscription(BzDownloader::subscription)
    .window_size((500.0, 800.0))
    .exit_on_close_request(false)
    .run_with(BzDownloader::new)
}

enum BzDownloader {
  // 加载历史下载任务
  // 等待FeedbackChannelCreated消息  用于接受任务下载时候反馈的信息
  Initializing(AppPreState),
  // 正常运行
  Running(AppState),
}

#[derive(Clone)]
pub struct AppPreState {
  tray_state: tray::TrayState,
  task_infos: Option<Vec<BzTaskInfo>>,
  feedback_sender: Option<tokio::sync::mpsc::Sender<BzTaskFeedBack>>,
}

impl AppPreState {
  pub fn is_ready(&self) -> bool {
    self.task_infos.is_some() && self.feedback_sender.is_some()
  }
}

struct AppState {
  tray_state: tray::TrayState,
  tasks: BTreeMap<BzTaskId, BzTask>,
  feedback_sender: tokio::sync::mpsc::Sender<BzTaskFeedBack>,
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
          runtime: BzTaskRuntimeInfo::default(),
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

#[derive(Debug, Clone)]
enum Message {
  // Initializing
  Loaded(Vec<BzTaskInfo>),
  FeedbackChannelCreated(tokio::sync::mpsc::Sender<BzTaskFeedBack>),

  // Running
  TrayMenuEvent(MenuEvent),
  TaskFeedBack(BzTaskFeedBack),
  BzTask(BzTaskMessage),
  WindowCloseRequest,
}

impl BzDownloader {
  fn new() -> (Self, Command<Message>) {
    let tray_state = tray::init_tray_icon();
    init_dirs();
    (
      Self::Initializing(AppPreState {
        tray_state,
        task_infos: None,
        feedback_sender: None,
      }),
      Command::perform(load_data(), Message::Loaded),
    )
  }

  fn update(&mut self, message: Message) -> Command<Message> {
    match self {
      BzDownloader::Initializing(app_pre_state) => {
        match message {
          Message::Loaded(mut task_infos) => {
            log::debug!("Loaded");
            app_pre_state.task_infos = Some(task_infos);
            if app_pre_state.is_ready() {
              *self =
                BzDownloader::Running(AppState::from(app_pre_state.clone()));
            }
          }
          Message::FeedbackChannelCreated(sender) => {
            log::debug!("FeedbackChannelCreated");
            app_pre_state.feedback_sender = Some(sender);
            if app_pre_state.is_ready() {
              *self =
                BzDownloader::Running(AppState::from(app_pre_state.clone()));
            }
          }
          _ => {
            log::error!(
              "Unexpected Message in state BzDownloader::Loading : {:?}",
              message
            );
          }
        }
        Command::none()
      }
      BzDownloader::Running(app_state) => {
        log::debug!("Message in update : {:?}", message);

        match message {
          Message::TrayMenuEvent(event) => {
            let menu_type = app_state.tray_state.menuids.get_type(&event.id);
            let cmd = match menu_type {
              BzMenuType::Display => {
                log::debug!("TrayMenuEvent: Display");
                window::get_latest().and_then(|window| {
                  window::change_mode(window, Mode::Windowed)
                })
              }
              BzMenuType::Hide => {
                log::debug!("TrayMenuEvent: Hide");
                window::get_latest()
                  .and_then(|window| window::change_mode(window, Mode::Hidden))
              }
              BzMenuType::Exit => {
                log::debug!("TrayMenuEvent: Exit");
                // 给每个worker发送退出消息
                // 等待所有worker退出
                // 退出前保存任务列表
                save_data(&app_state.tasks);
                window::get_latest().and_then(window::close)
              }
              _ => Command::none(),
            };
            cmd
          }
          Message::WindowCloseRequest => {
            log::debug!("WindowCloseRequest in App. Just Hide Window");
            window::get_latest()
              .and_then(|window| window::change_mode(window, Mode::Hidden))
          }

          Message::BzTask(task_meaasge) => {
            log::debug!("BzTaskMessage: {:?}", task_meaasge);
            match task_meaasge {
              bz_task::BzTaskMessage::AddTask(task_info) => {
                log::debug!("AddTask: {:?}", task_info);
                let mut task = BzTask::from_info(task_info);
                let control_sender = bz_task::run_task(
                  task.id,
                  task.info.clone(),
                  app_state.feedback_sender.clone(),
                );
                task.runtime.sender = Some(control_sender);
                app_state.tasks.insert(task.id, task);
              }
              _ => {}
            }
            Command::none()
          }
          Message::TaskFeedBack(feedback) => {
            let task_id = feedback.task_id;
            let progress = feedback.progress;

            app_state.tasks.get_mut(&task_id).map(|task| {
              task.extra.progress = progress;
            });
            Command::none()
          }
          _ => Command::none(),
        }
      }
    }
  }

  fn view(&self) -> Element<Message> {
    match self {
      BzDownloader::Initializing(_) => Element::new(Text::new("Loading...")),
      BzDownloader::Running(app_state) => {
        let header = self.view_header();
        let h = horizontal_rule(5);
        let body = self.view_body(app_state);
        column![header, h, body].spacing(10).padding(30).into()
      }
    }
  }

  fn subscription(&self) -> Subscription<Message> {
    let tray_subscription = Subscription::run(tray::tray_subscription);
    let window_close_requests = window::close_requests().map(|id| {
      log::debug!("WindowClose in window_close_requests");
      Message::WindowCloseRequest
    });

    let task_feedback_subscription =
      Subscription::run(bz_task::feed_back_subscription);

    Subscription::batch(vec![
      tray_subscription,
      window_close_requests,
      task_feedback_subscription,
    ])
  }
}

use directories::ProjectDirs;

fn App_Dir() -> ProjectDirs {
  let app_dir = ProjectDirs::from("com", "breezing", "bz_downloader").unwrap();
  return app_dir;
}

fn init_dirs() {
  let app_dir = App_Dir();
  let cache_dir = app_dir.cache_dir();
  let data_dir = app_dir.data_dir();
  log::debug!("cache_dir: {:?}", cache_dir);
  log::debug!("data_dir: {:?}", data_dir);
  if !cache_dir.exists() {
    log::info!("cache_dir not exists");
    log::info!("create cache_dir: {}", cache_dir.display());
    std::fs::create_dir_all(cache_dir).unwrap();
  }
  if !data_dir.exists() {
    log::info!("data_dir not exists");
    log::info!("create data_dir: {}", data_dir.display());
    std::fs::create_dir_all(data_dir).unwrap();
  }
}

async fn load_data() -> Vec<BzTaskInfo> {
  let task_list = App_Dir().data_dir().join("task_list.json");
  if !task_list.exists() {
    return Vec::new();
  }
  let task_infos: Vec<BzTaskInfo> =
    serde_json::from_reader(std::fs::File::open(task_list).unwrap()).unwrap();
  task_infos
}

async fn save_data(tasks: &BTreeMap<BzTaskId, BzTask>) {
  let task_list = App_Dir().data_dir().join("task_list.json");
  let task_infos: Vec<&BzTaskInfo> =
    tasks.values().map(|task| &task.info).collect();
  serde_json::to_writer(std::fs::File::create(task_list).unwrap(), &task_infos)
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

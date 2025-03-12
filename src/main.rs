mod bz_task;
mod error;
mod m3u8;
mod tray;
mod view;
mod zfs;

use bz_task::{
  BzTask, BzTaskExtraInfo, BzTaskId, BzTaskInfoFeedBackMessage, BzTaskMessage, BzTaskRuntimeInfo
};
use bz_task::{BzTaskFeedBack, BzTaskInfo, BzTaskStatus, BzTaskType};
use error::{BzError, BzResult};
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
  // TODO 把日志显示在界面上
  env_logger::Builder::new()
    .filter_module("bz_downloader", log::LevelFilter::Debug)
    .init();

  let font_bytes = include_bytes!("../resource/MicrosoftYaHei-01.ttf");
  let font = Font::with_name("微软雅黑");
  iced::application("BzDownloader", BzDownloader::update, BzDownloader::view)
    .subscription(BzDownloader::subscription)
    .window_size((1000.0, 700.0))
    .exit_on_close_request(false)
    .font(font_bytes)
    .default_font(font)
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

#[derive(Debug, Clone)]
enum Message {
  // Initializing
  Loaded(Vec<BzTaskInfo>),
  FeedbackChannelCreated(tokio::sync::mpsc::Sender<BzTaskFeedBack>),

  // Running
  TrayMenuEvent(MenuEvent),
  TaskInfoFeedBack(BzTaskInfoFeedBackMessage),
  BzTask(BzTaskMessage),
  WindowCloseRequest,
  SaveCompleted, //真正的关闭
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
          Message::Loaded(task_infos) => {
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
        log::debug!("[update] : {:?}", message);
        let res = deal_running_message(app_state, message);
        match res {
          Ok(cmd) => cmd,
          Err(err) => {
            log::error!("Error in deal_running_message: {:?}", err);
            Command::none()
          }
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

fn deal_initializing_message(
  app_pre_state: &mut AppPreState, message: Message,
) -> BzResult<Command<Message>> {
  Ok(Command::none())
}

fn deal_running_message(
  app_state: &mut AppState, message: Message,
) -> BzResult<Command<Message>> {
  let cmd = match message {
    Message::TrayMenuEvent(event) => {
      let res = deal_trayevent(app_state, event);
      res?
    }
    Message::SaveCompleted => {
      log::debug!("SaveCompleted");
      window::get_latest().and_then(window::close)
    }
    Message::WindowCloseRequest => {
      log::debug!("WindowCloseRequest in App. Just Hide Window");
      window::get_latest()
        .and_then(|window| window::change_mode(window, Mode::Hidden))
    }

    Message::BzTask(task_meaasge) => {
      log::debug!("[Message::BzTask] BzTaskMessage: {:?}", task_meaasge);
      let res = bz_task::message::deal_bztask_message(app_state, task_meaasge);
      Command::none()
    }
    Message::TaskInfoFeedBack(feedback) => {
      let task_id = feedback.task_id;
      let progress = feedback.progress;

      app_state.tasks.get_mut(&task_id).map(|task| {
        task.extra.progress = progress;
      });
      Command::none()
    }
    _ => Command::none(),
  };
  Ok(cmd)
}

fn deal_trayevent(
  app_state: &mut AppState, event: MenuEvent,
) -> BzResult<Command<Message>> {
  let menu_type = app_state.tray_state.menuids.get_type(&event.id);
  let cmd = match menu_type {
    BzMenuType::Display => {
      log::debug!("TrayMenuEvent: Display");
      window::get_latest()
        .and_then(|window| window::change_mode(window, Mode::Windowed))
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
      let task_infos: Vec<BzTaskInfo> = app_state
        .tasks
        .values()
        .map(|task| task.info.clone())
        .collect();
      Command::perform(save_data(task_infos), |_| Message::SaveCompleted)
    }
    _ => Command::none(),
  };

  Ok(cmd)
}

use directories::ProjectDirs;

fn App_Dir() -> ProjectDirs {
  let app_dir = ProjectDirs::from("com", "breezing", "bz_downloader").unwrap();
  return app_dir;
}

fn init_dirs() {
  let app_dir = App_Dir();
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

async fn load_data() -> Vec<BzTaskInfo> {
  let task_list = App_Dir().data_local_dir().join("task_list.json");
  if !task_list.exists() {
    return Vec::new();
  }
  let task_infos: Vec<BzTaskInfo> =
    serde_json::from_reader(std::fs::File::open(task_list).unwrap()).unwrap();
  task_infos
}

async fn save_data(task_infos: Vec<BzTaskInfo>) {
  let task_list = App_Dir().data_local_dir().join("task_list.json");
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

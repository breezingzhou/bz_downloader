mod bz_task;
mod error;
mod m3u8;
mod tray;
mod view;
mod zfs;

use bz_task::BzTaskMessage;
use bz_task::{BzTaskFeedBack, BzTaskInfo, BzTaskType, TaskStatus};
use error::BzError;
use iced::{
  Element, Font, Subscription, Task as Command, keyboard,
  widget::{Text, column, horizontal_rule},
  window::{self, Mode},
};
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
  tasks: Option<Vec<BzTaskInfo>>,
  feedback_sender: Option<tokio::sync::mpsc::Sender<BzTaskFeedBack>>,
}

impl AppPreState {
  pub fn is_ready(&self) -> bool {
    self.tasks.is_some() && self.feedback_sender.is_some()
  }
}

struct AppState {
  tray_state: tray::TrayState,
  tasks: Vec<BzTaskInfo>,
  feedback_sender: tokio::sync::mpsc::Sender<BzTaskFeedBack>,
}

impl From<AppPreState> for AppState {
  fn from(app_pre_state: AppPreState) -> Self {
    Self {
      tray_state: app_pre_state.tray_state,
      tasks: app_pre_state.tasks.unwrap(),
      feedback_sender: app_pre_state.feedback_sender.unwrap(),
    }
  }
}

#[derive(Debug, Clone)]
enum Message {
  // Initializing
  Loaded(String),
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
    (
      Self::Initializing(AppPreState {
        tray_state,
        tasks: None,
        feedback_sender: None,
      }),
      Command::perform(load_data(), Message::Loaded),
    )
  }

  fn update(&mut self, message: Message) -> Command<Message> {
    match self {
      BzDownloader::Initializing(app_pre_state) => {
        match message {
          Message::Loaded(s) => {
            log::debug!("Loaded");
            app_pre_state.tasks = Some(Vec::new());
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
              bz_task::BzTaskMessage::AddTask(url) => {
                log::debug!("AddTask: {:?}", url);
                let task_info = BzTaskInfo {
                  id: 0,
                  src: url.parse().unwrap(),
                  dest: "./tmp/1.mp4".into(),
                  cache: "./tmp/cache".into(),
                  kind: BzTaskType::Zfs,
                  status: TaskStatus::Queued,
                };
                app_state.tasks.push(task_info.clone());
                bz_task::run_task(task_info, app_state.feedback_sender.clone());
              }
              _ => {}
            }
            Command::none()
          }
          Message::TaskFeedBack(feedback_message) => {
            let progress = feedback_message.progress;
            app_state.tasks[0].status = TaskStatus::Downloading;
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

async fn load_data() -> String {
  async_std::task::sleep(Duration::from_secs(2)).await;
  return "load_data finish".to_string();
}

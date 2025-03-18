use crate::app_state::{AppPreState, AppState};
use crate::bz_task::{BzTaskFeedBack, BzTaskInfo};
use crate::bz_task::{BzTaskInfoFeedBackMessage, BzTaskMessage};
use crate::error::BzResult;
use crate::tray::{self, BzMenuType};
use iced::{
  Element, Subscription, Task as Command,
  widget::{Text, column, horizontal_rule},
  window::{self, Mode},
};
use tray_icon::menu::MenuEvent;

#[derive(Debug, Clone)]
pub enum Message {
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

pub enum BzDownloader {
  // 加载历史下载任务
  // 等待FeedbackChannelCreated消息  用于接受任务下载时候反馈的信息
  Initializing(AppPreState),
  // 正常运行
  Running(AppState),
}

impl BzDownloader {
  pub fn new() -> (Self, Command<Message>) {
    let tray_state = tray::init_tray_icon();
    crate::app_state::init_dirs();
    (
      Self::Initializing(AppPreState {
        tray_state,
        task_infos: None,
        feedback_sender: None,
      }),
      Command::perform(crate::app_state::load_data(), Message::Loaded),
    )
  }

  pub fn update(&mut self, message: Message) -> Command<Message> {
    match self {
      BzDownloader::Initializing(app_pre_state) => {
        log::debug!("[initializing] : {:?}", message);
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

  pub fn view(&self) -> Element<Message> {
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

  pub fn subscription(&self) -> Subscription<Message> {
    let tray_subscription = Subscription::run(tray::tray_subscription);
    let window_close_requests = window::close_requests().map(|_id| {
      log::debug!("WindowClose in window_close_requests");
      Message::WindowCloseRequest
    });

    let task_feedback_subscription =
      Subscription::run(crate::bz_task::feed_back_subscription);

    Subscription::batch(vec![
      tray_subscription,
      window_close_requests,
      task_feedback_subscription,
    ])
  }
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
      let _ = crate::bz_task::deal_bztask_message(app_state, task_meaasge);
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
      Command::perform(crate::app_state::save_data(task_infos), |_| {
        Message::SaveCompleted
      })
    }
    _ => Command::none(),
  };

  Ok(cmd)
}

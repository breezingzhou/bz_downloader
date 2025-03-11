use iced::{
  Element, Font, Subscription, Task as Command, keyboard,
  widget::{Text, column, horizontal_rule},
  window::{self, Mode},
};
use std::{env::set_var, sync::Arc, time::Duration};
use tray::BzMenuType;
use tray_icon::{TrayIcon, menu::MenuEvent};

mod bz_task;
mod m3u8;
mod tray;
mod view;

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
  Loading(tray::TrayState),

  Loaded(AppState),
}

struct AppState {
  tray_state: tray::TrayState,
  tasks: Vec<bz_task::BzTaskInfo>,
}

#[derive(Debug, Clone)]
enum Message {
  Loaded(String),
  TrayMenuEvent(MenuEvent),
  BzTask(bz_task::BzTaskMessage),
  WindowCloseRequest,
  ToggleFullscreen(window::Mode),
}

impl BzDownloader {
  fn new() -> (Self, Command<Message>) {
    let tray_state = tray::init_tray_icon();
    (
      Self::Loading(tray_state),
      Command::perform(load_data(), Message::Loaded),
    )
  }

  fn update(&mut self, message: Message) -> Command<Message> {
    match self {
      BzDownloader::Loading(tray_state) => match message {
        Message::Loaded(s) => {
          log::debug!("Loaded");
          log::debug!("msg from load_data: {}", s);
          let app_state = AppState {
            tray_state: tray_state.clone(),
            tasks: vec![],
          };
          *self = BzDownloader::Loaded(app_state);
          Command::none()
        }

        _ => Command::none(),
      },
      BzDownloader::Loaded(app_state) => {
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
            log::debug!("WindowClose in App ");
            window::get_latest()
              .and_then(|window| window::change_mode(window, Mode::Hidden))
          }
          Message::ToggleFullscreen(mode) => {
            log::debug!("ToggleFullscreen: {:?}", mode);
            Command::none()
          }
          Message::BzTask(task_meaasge) => {
            log::debug!("BzTaskMessage: {:?}", task_meaasge);
            match task_meaasge {
              bz_task::BzTaskMessage::AddTask(url) => {
                log::debug!("AddTask: {:?}", url);
                app_state.tasks.push(bz_task::BzTaskInfo {
                  src: url.parse().unwrap(),
                  dest: "./tmp/1.mp4".into(),
                  temp: "./tmp/cache".into(),
                  status: bz_task::TaskStatus::Queued,
                });
              }
              _ => {}
            }
            Command::none()
          }
          _ => Command::none(),
        }
      }
    }
  }

  fn view(&self) -> Element<Message> {
    match self {
      BzDownloader::Loading(_) => Element::new(Text::new("Loading...")),
      BzDownloader::Loaded(app_state) => {
        let header = self.view_header();
        let h = horizontal_rule(5);
        let body = self.view_body(app_state);
        column![header, h, body].spacing(10).padding(30).into()
      }
    }
  }

  fn subscription(&self) -> Subscription<Message> {
    use keyboard::key;

    let keyboard_subscription = keyboard::on_key_press(|key, modifiers| {
      let keyboard::Key::Named(key) = key else {
        return None;
      };

      match (key, modifiers) {
        (key::Named::ArrowUp, keyboard::Modifiers::SHIFT) => {
          Some(Message::ToggleFullscreen(window::Mode::Fullscreen))
        }
        (key::Named::ArrowDown, keyboard::Modifiers::SHIFT) => {
          Some(Message::ToggleFullscreen(window::Mode::Windowed))
        }
        _ => None,
      }
    });
    let tray_subscription = Subscription::run(tray::tray_subscription);
    let window_close_requests = window::close_requests().map(|id| {
      log::debug!("WindowClose in window_close_requests");
      Message::WindowCloseRequest
    });

    Subscription::batch(vec![
      keyboard_subscription,
      tray_subscription,
      window_close_requests,
    ])
  }
}

async fn load_data() -> String {
  async_std::task::sleep(Duration::from_secs(2)).await;
  return "load_data finish".to_string();
}

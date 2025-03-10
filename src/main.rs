use iced::{
  Element, Subscription, Task as Command, keyboard,
  widget::Text,
  window::{self, Mode},
};
use std::{env::set_var, sync::Arc, time::Duration};
use tray::BzMenuType;
use tray_icon::{TrayIcon, menu::MenuEvent};

mod bz_task;
mod m3u8;
mod tray;

pub fn main() -> iced::Result {
  env_logger::init();

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

#[derive(Debug)]
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
          println!("Loaded");
          println!("msg from load_data: {}", s);
          let app_state = AppState {
            tray_state: tray_state.clone(),
            tasks: vec![],
          };
          *self = BzDownloader::Loaded(app_state);
          Command::none()
        }

        _ => Command::none(),
      },
      BzDownloader::Loaded(app_state) => match message {
        Message::TrayMenuEvent(event) => {
          let menu_type = app_state.tray_state.menuids.get_type(&event.id);
          let cmd = match menu_type {
            BzMenuType::Display => {
              println!("TrayMenuEvent: Display");
              window::get_latest()
                .and_then(|window| window::change_mode(window, Mode::Windowed))
            }
            BzMenuType::Hide => {
              println!("TrayMenuEvent: Hide");
              window::get_latest()
                .and_then(|window| window::change_mode(window, Mode::Hidden))
            }
            BzMenuType::Exit => {
              println!("TrayMenuEvent: Exit");
              window::get_latest().and_then(window::close)
            }
            _ => Command::none(),
          };
          cmd
        }
        Message::WindowCloseRequest => {
          println!("WindowClose in App ");
          window::get_latest()
            .and_then(|window| window::change_mode(window, Mode::Hidden))
        }

        Message::ToggleFullscreen(mode) => {
          println!("ToggleFullscreen: {:?}", mode);
          Command::none()
        }
        _ => Command::none(),
      },
    }
  }

  fn view(&self) -> Element<Message> {
    match self {
      BzDownloader::Loading(_) => Element::new(Text::new("Loading...")),
      BzDownloader::Loaded(_) => Element::new(Text::new("Loaded...")),
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
      println!("WindowClose in window_close_requests");
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

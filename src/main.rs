use std::{fmt::Error, time::Duration};

use iced::{
  Element, Subscription, Task as Command, keyboard, widget::Text, window,
};

pub fn main() -> iced::Result {
  env_logger::init();
  iced::application("BzDownloader", BzDownloader::update, BzDownloader::view)
    .subscription(BzDownloader::subscription)
    .window_size((500.0, 800.0))
    .run_with(BzDownloader::new)
}

#[derive(Debug)]
enum BzDownloader {
  Loading,
  Loaded(AppState),
}

#[derive(Debug)]
enum TaskStatus {
  Queued,
  Downloading,
  Paused,
  Completed,
  Failed,
}

#[derive(Debug)]
struct DownloadTask {
  url: String,
  progress: f32,
  status: TaskStatus,
}

#[derive(Debug)]
struct AppState {
  tasks: Vec<DownloadTask>,
}

impl Default for AppState {
  fn default() -> Self {
    return Self { tasks: Vec::new() };
  }
}

#[derive(Debug)]
enum Message {
  Loaded(String),
  AddTask(String),
  PauseTask(usize),
  RestartTask(usize),
  RemoveTask(usize),
  ToggleFullscreen(window::Mode),
}

impl BzDownloader {
  fn new() -> (Self, Command<Message>) {
    (
      Self::Loading,
      Command::perform(load_data(), Message::Loaded),
    )
  }

  fn update(&mut self, message: Message) -> Command<Message> {
    match message {
      Message::Loaded(s) => {
        println!("Loaded");
        println!("msg from load_data: {}", s);
        *self = Self::Loaded(AppState::default());
        Command::none()
      }
      Message::AddTask(url) => {
        println!("Downloading: {}", url);
        Command::none()
      }
      _ => Command::none(),
    }
  }

  fn view(&self) -> Element<Message> {
    match self {
      Self::Loading => Element::new(Text::new("Loading...")),
      Self::Loaded(_state) => Element::new(Text::new("Loaded...")),
    }
  }

  fn subscription(&self) -> Subscription<Message> {
    use keyboard::key;

    keyboard::on_key_press(|key, modifiers| {
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
    })
  }
}

async fn load_data() -> String {
  async_std::task::sleep(Duration::from_secs(2)).await;
  return "load_data finish".to_string();
}

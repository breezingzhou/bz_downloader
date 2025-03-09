use iced::Event;
use iced::futures::stream::{Stream, StreamExt};
use iced::{
  Element, Subscription, Task as Command,
  futures::SinkExt,
  keyboard, stream,
  widget::{Text, button},
  window,
};
use std::arch::x86_64::_CMP_FALSE_OQ;
use std::default;
use std::io::Write;
use std::{fmt::Error, time::Duration};
use tray_icon::TrayIcon;
use tray_icon::menu::PredefinedMenuItem;
use tray_icon::{
  TrayIconBuilder,
  menu::{Menu, MenuItem},
};

fn new_tray_menu() -> Menu {
  let tray_menu = Menu::new();
  let item1 = MenuItem::new("显示", true, None);
  let item2 = PredefinedMenuItem::quit(Some("退出"));
  tray_menu.append(&item1).unwrap();
  tray_menu.append(&item2).unwrap();
  tray_menu
}

pub fn main() -> iced::Result {
  env_logger::init();

  let res =
    iced::application("BzDownloader", BzDownloader::update, BzDownloader::view)
      .subscription(BzDownloader::subscription)
      .window_size((500.0, 800.0))
      .exit_on_close_request(false)
      .run_with(BzDownloader::new);

  println!("Hello, world!");
  res
}

struct BzDownloader {
  is_loading: bool,
  tray_icon: Option<TrayIcon>,
  state: AppState,
}

impl Default for BzDownloader {
  fn default() -> Self {
    Self {
      is_loading: true,
      tray_icon: None,
      state: AppState::default(),
    }
  }
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
  TrayIconEvent,
  AddTask(String),
  PauseTask(usize),
  RestartTask(usize),
  RemoveTask(usize),
  WindowCloseRequest,
  ToggleFullscreen(window::Mode),
}

impl BzDownloader {
  fn new() -> (Self, Command<Message>) {
    let tray_icon = TrayIconBuilder::new()
      .with_menu(Box::new(new_tray_menu()))
      .with_tooltip("BzDownloader")
      // .with_icon(icon)
      .build()
      .unwrap();
    (
      Self {
        is_loading: true,
        tray_icon: Some(tray_icon),
        state: AppState::default(),
      },
      Command::perform(load_data(), Message::Loaded),
    )
  }

  fn update(&mut self, message: Message) -> Command<Message> {
    match message {
      Message::Loaded(s) => {
        println!("Loaded");
        println!("msg from load_data: {}", s);
        self.is_loading = false;
        Command::none()
      }
      Message::TrayIconEvent => {
        println!("TrayIconEvent in App ");
        Command::none()
      }
      Message::WindowCloseRequest => {
        println!("WindowClose in App");

        window::get_latest().and_then(window::close)
      }
      Message::AddTask(url) => {
        println!("Downloading: {}", url);
        Command::none()
      }
      Message::ToggleFullscreen(mode) => {
        println!("ToggleFullscreen: {:?}", mode);
        Command::none()
      }
      _ => Command::none(),
    }
  }

  fn view(&self) -> Element<Message> {
    match self.is_loading {
      true => Element::new(Text::new("Loading...")),
      false => Element::new(Text::new("Loaded...")),
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
    let tray_subscription = Subscription::run(trayicon);
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

fn trayicon() -> impl Stream<Item = Message> {
  stream::channel(100, |mut output| async move {
    loop {
      if let Ok(event) = tray_icon::TrayIconEvent::receiver().try_recv() {
        match event {
          tray_icon::TrayIconEvent::Click { .. } => {
            println!("TrayIconEvent: click",);
            let r = output.send(Message::TrayIconEvent).await.unwrap();
          }
          _ => {}
        };
      }
      async_std::task::sleep(Duration::from_millis(100)).await;
    }
  })
}

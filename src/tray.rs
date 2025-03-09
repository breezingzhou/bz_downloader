use std::collections::HashMap;
use std::str::FromStr as _;
use std::time::Duration;

use iced::futures::SinkExt;
use iced::futures::stream::{Stream, StreamExt};
use tray_icon::TrayIcon;
use tray_icon::menu::MenuItemBuilder;
use tray_icon::{
  TrayIconBuilder,
  menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
};

use crate::Message;

#[derive(Debug)]

pub enum TrayMessage {
  Quit,
  Display,
  Hide,
}

#[derive(Clone)]
pub struct TrayState {
  pub tray_icon: TrayIcon,
  pub menuids: MenuIdCollection,
}

#[derive(Clone)]
pub enum BzMenuType {
  Display,
  Hide,
  Exit,
  Unknown,
}

#[derive(Clone)]
pub struct MenuIdCollection {
  menuids2type: HashMap<MenuId, BzMenuType>,
}

impl MenuIdCollection {
  pub fn new() -> Self {
    Self {
      menuids2type: HashMap::new(),
    }
  }

  pub fn insert(&mut self, id: MenuId, menu_type: BzMenuType) {
    self.menuids2type.insert(id, menu_type);
  }

  pub fn get_type(&self, id: &MenuId) -> &BzMenuType {
    self.menuids2type.get(id).unwrap_or(&BzMenuType::Unknown)
  }
}

pub fn init_tray_menu() -> (Menu, MenuIdCollection) {
  let tray_menu = Menu::new();
  let item1 = MenuItem::new("显示", true, None);
  let item2 = MenuItem::new("隐藏", true, None);
  let item3 = MenuItem::new("退出", true, None);

  tray_menu.append(&item1).unwrap();
  tray_menu.append(&item2).unwrap();
  tray_menu.append(&item3).unwrap();
  let mut menuids = MenuIdCollection::new();
  menuids.insert(item1.id().clone(), BzMenuType::Display);
  menuids.insert(item2.id().clone(), BzMenuType::Hide);
  menuids.insert(item3.id().clone(), BzMenuType::Exit);
  (tray_menu, menuids)
}

pub fn init_tray_icon() -> TrayState {
  let (tray_menu, menuids) = init_tray_menu();
  let tray_icon = TrayIconBuilder::new()
    .with_menu(Box::new(tray_menu.clone()))
    .with_tooltip("BzDownloader")
    // .with_icon(icon)
    .build()
    .unwrap();
  TrayState { tray_icon, menuids }
}

pub fn tray_subscription() -> impl Stream<Item = Message> {
  iced::stream::channel(100, |mut output| async move {
    loop {
      if let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
        println!("MenuEvent in Subscription! event : {:?}", event);
        let _ = output.send(Message::TrayMenuEvent(event)).await;
      } else {
        async_std::task::sleep(Duration::from_millis(100)).await;
      }
    }
  })
}

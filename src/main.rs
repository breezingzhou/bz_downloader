mod app_state;
mod bz_downloader;
mod bz_task;
mod error;
mod m3u8;
mod tray;
mod view;
mod zfs;

use bz_downloader::BzDownloader;
use iced::Font;

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

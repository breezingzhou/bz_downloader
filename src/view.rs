use iced::widget::{button, column, row, text, vertical_rule};
use reqwest::Url;

use crate::{
  AppState, Message,
  bz_task::{
    self, BzTask, BzTaskInfo, BzTaskMessage, BzTaskStatus, BzTaskType,
  },
};

impl crate::BzDownloader {
  pub fn view_header(&self) -> iced::Element<Message> {
    let task_info = BzTaskInfo {
      src: Url::parse("https://svipsvip.ffzy-online5.com/20250118/37333_517b17a8/2000k/hls/mixed.m3u8").unwrap(),
      dest: "./tmp/1.mp4".into(),
        cache: "./tmp".into(),
        kind: BzTaskType::M3u8,
        status: BzTaskStatus::Queued,
    };
    let message = Message::BzTask(BzTaskMessage::AddTask(task_info));
    let button = button("+").on_press(message);
    row![button].into()
  }

  pub fn view_body(&self, app_state: &AppState) -> iced::Element<Message> {
    let filter = self.view_filter();
    let v = vertical_rule(10);

    let tasks = self.view_tasks(app_state);
    row![filter, v, tasks].spacing(30).into()
  }

  pub fn view_tasks(&self, app_state: &AppState) -> iced::Element<Message> {
    let mut tasks_view = column![];
    for task in app_state.tasks.values() {
      let task_view = self.view_task(task);
      tasks_view = tasks_view.push(task_view);
    }
    tasks_view.into()
  }

  pub fn view_task(&self, task: &BzTask) -> iced::Element<Message> {
    let name = task.info.dest.file_name().unwrap().to_str().unwrap();
    let name_view = text!("{name}");

    let status = format!("{:?}", task.info.status);
    let status_view = text!("{status}");
    row![name_view, status_view].into()
  }

  pub fn view_filter(&self) -> iced::Element<Message> {
    // TODO shaping(text::Shaping::Advanced) 是为了显示中文 可能有其他方案
    let button_all = button(text!("全部").shaping(text::Shaping::Advanced));
    let button_downloading =
      button(text!("进行中").shaping(text::Shaping::Advanced));
    let button_init = button(text!("未开始").shaping(text::Shaping::Advanced));
    let button_finish =
      button(text!("已完成").shaping(text::Shaping::Advanced));
    let button_error = button(text!("错误").shaping(text::Shaping::Advanced));
    column![
      button_all,
      button_downloading,
      button_init,
      button_finish,
      button_error
    ]
    .into()
  }
}

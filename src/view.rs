use iced::{
  Element,
  Length::FillPortion,
  widget::{
    Container, Text, button, column, container, horizontal_rule, progress_bar,
    row, text, vertical_rule,
  },
};
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
    let taskinfo_header = row![
      text!("任务").width(FillPortion(3)),
      vertical_rule(5),
      text!("状态").width(FillPortion(1)),
      vertical_rule(5),
      text!("进度").width(FillPortion(1)),
      vertical_rule(5),
      text!("操作").width(FillPortion(3))
    ];
    tasks_view = tasks_view.push(taskinfo_header.height(iced::Length::Shrink));
    tasks_view = tasks_view.push(horizontal_rule(5));
    for task in app_state.tasks.values() {
      let task_view = self.view_task(task);
      tasks_view = tasks_view.push(task_view);
      tasks_view = tasks_view.push(horizontal_rule(5))
    }
    tasks_view.into()
  }

  pub fn view_task(&self, task: &BzTask) -> iced::Element<Message> {
    let name = task.info.dest.file_name().unwrap().to_str().unwrap();
    let name_view = text!("{name}").width(FillPortion(3));

    let status = format!("{}", task.info.status);
    let status_view = text!("{status}").width(FillPortion(1));

    let progress_view =
      progress_bar(0.0..=1.0, task.extra.progress).width(FillPortion(1));

    let action_view = self.view_task_action(task).width(FillPortion(3));
    row![
      name_view,
      vertical_rule(5),
      status_view,
      vertical_rule(5),
      progress_view,
      vertical_rule(5),
      action_view
    ]
    .height(iced::Length::Shrink)
    .into()
  }

  pub fn view_task_action(&self, task: &BzTask) -> Container<Message> {
    let button_start = button(text!("开始"))
      .on_press(Message::BzTask(BzTaskMessage::TryStartTask(task.id)));
    let button_stop = button(text!("暂停"))
      .on_press(Message::BzTask(BzTaskMessage::TryStopTask(task.id)));
    let button_remove = button(text!("删除"))
      .on_press(Message::BzTask(BzTaskMessage::RemoveTask(task.id)));
    let buttons = match task.info.status {
      BzTaskStatus::Queued => Vec::from([button_start, button_remove]),
      BzTaskStatus::Running => Vec::from([button_stop]),
      BzTaskStatus::Stopped => Vec::from([button_start, button_remove]),
      BzTaskStatus::Completed => Vec::from([button_remove]),
      BzTaskStatus::Failed => Vec::from([button_start, button_remove]),
    };
    container(row(
      buttons.into_iter().map(Element::from).collect::<Vec<_>>(),
    ))
    .padding(3)
  }

  pub fn view_filter(&self) -> iced::Element<Message> {
    let button_all = button(text!("全部"));
    let button_downloading = button(text!("进行中"));
    let button_init = button(text!("未开始"));
    let button_finish = button(text!("已完成"));
    let button_error = button(text!("错误"));
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

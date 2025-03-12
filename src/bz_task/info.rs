use std::path::PathBuf;

use reqwest::Url;
use serde::{Deserialize, Serialize, ser::SerializeStruct as _};

use super::BzTaskId;

// 用于展示和存储的状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BzTaskStatus {
  Queued,
  Downloading,
  Paused,
  Completed,
  Failed,
}

// worker中任务的状态
pub enum TaskInnerStatus {
  Started,
  Paused,
  Stopped,
  Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]

pub enum BzTaskType {
  M3u8,
  Zfs,
}

pub struct BzTask {
  pub id: BzTaskId,
  pub info: BzTaskInfo,
  pub extra: BzTaskExtraInfo,
  pub runtime: Option<BzTaskRuntimeInfo>
}

// 直接传递给各个worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BzTaskInfo {
  // #[serde(skip_serializing)]
  // #[serde(default = "BzTaskId::zero")]
  // pub id: BzTaskId, // 系统内部使用 不会被dump 每次启动程序从0开始累加
  #[serde(
    serialize_with = "serialize_url",
    deserialize_with = "deserialize_url"
  )]
  pub src: Url,
  pub dest: PathBuf,  // 下载目录
  pub cache: PathBuf, // 临时文件
  pub kind: BzTaskType,
  pub status: BzTaskStatus,
  // 创建时间 完成时间等
  // TODO 简易的序列化和反序列化
}

// 界面上需要展示的内容 worker通过channel发送过来的？
#[derive(Debug, Clone, Default)]
pub struct BzTaskExtraInfo {
  pub progress: f32,
  pub current_size: u64,
  pub total_size: u64,
}

#[derive(Debug,)]
pub struct BzTaskRuntimeInfo {
  pub sender: tokio::sync::mpsc::Sender<BzTaskControl>,
  pub join_handle: tokio::task::JoinHandle<()>,
}


// ==============================================

impl BzTask {
  pub fn from_info(info: BzTaskInfo) -> Self {
    Self {
      id: BzTaskId::unique(),
      info,
      extra: BzTaskExtraInfo::default(),
      runtime: None,
    }
  }
}
// ==============================================

#[derive(Debug, Clone)]
pub enum Control {
  Pause,
  Restart,
  Stop,
}

#[derive(Debug, Clone)]
pub struct BzTaskControl {
  pub id: usize,
  pub control: Control,
}

#[derive(Debug, Clone)]
pub struct BzTaskFeedBack {
  pub task_id: BzTaskId,
  pub progress: f32,
}

// ==============================================

fn serialize_url<S>(url: &Url, serializer: S) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  serializer.serialize_str(&url.to_string())
}

fn deserialize_url<'de, D>(deserializer: D) -> Result<Url, D::Error>
where
  D: serde::Deserializer<'de>,
{
  let s = String::deserialize(deserializer)?;
  Url::parse(&s).map_err(serde::de::Error::custom)
}

// impl Serialize for BzTaskInfo {
//   fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//   where
//     S: serde::Serializer,
//   {
//     let mut s = serializer.serialize_struct("BzTaskInfo", 4)?;
//     s.serialize_field("src", &self.src.to_string());
//     s.serialize_field("dest", &self.dest);
//     s.serialize_field("cache", &self.cache);
//     s.serialize_field("status", &self.status);
//     s.end()
//   }
// }

// impl<'de> Deserialize<'de> for BzTaskInfo {
//   fn deserialize<D>(deserializer: D) -> Result<BzTaskInfo, D::Error>
//   where
//     D: serde::Deserializer<'de>,
//   {
//     struct BzTaskInfoVisitor;

//     impl<'de> serde::de::Visitor<'de> for BzTaskInfoVisitor {
//       type Value = BzTaskInfo;

//       fn expecting(
//         &self, formatter: &mut std::fmt::Formatter,
//       ) -> std::fmt::Result {
//         formatter.write_str("struct BzTaskInfo")
//       }

//       fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
//       where
//         A: serde::de::MapAccess<'de>,
//       {
//         let mut src = None;
//         let mut dest = None;
//         let mut cache = None;
//         let mut kind = None;
//         let mut status = None;
//         while let Some(key) = map.next_key::<String>()? {
//           match key.as_str() {
//             "src" => {
//               if src.is_some() {
//                 return Err(serde::de::Error::duplicate_field("src"));
//               }
//               src = Url::parse(map.next_value()?).ok();
//             }
//             "dest" => {
//               if dest.is_some() {
//                 return Err(serde::de::Error::duplicate_field("dest"));
//               }
//               dest = Some(PathBuf::from(map.next_value::<String>()?))
//             }
//             "cache" => {
//               if cache.is_some() {
//                 return Err(serde::de::Error::duplicate_field("cache"));
//               }
//               cache = Some(PathBuf::from(map.next_value::<String>()?))
//             }
//             "status" => {
//               if status.is_some() {
//                 return Err(serde::de::Error::duplicate_field("status"));
//               }
//               status = Some(TaskStatus::from(map.next_value()?))
//             }
//             "kind" => {
//               if status.is_some() {
//                 return Err(serde::de::Error::duplicate_field("kind"));
//               }
//               kind = Some(BzTaskType::from(map.next_value()?))
//             }

//             _ => {
//               // 忽略未知字段
//               let _: serde_json::Value = map.next_value()?;
//             }
//           }
//         }
//         let src = src.ok_or_else(|| serde::de::Error::missing_field("src"))?;
//         let dest =
//           dest.ok_or_else(|| serde::de::Error::missing_field("dest"))?;
//         let cache =
//           cache.ok_or_else(|| serde::de::Error::missing_field("cache"))?;
//         let kind =
//           kind.ok_or_else(|| serde::de::Error::missing_field("kind"))?;
//         let status =
//           status.ok_or_else(|| serde::de::Error::missing_field("status"))?;

//         Ok(BzTaskInfo {
//           id: 0,
//           src,
//           dest,
//           cache,
//           kind,
//           status,
//         })
//       }
//     }

//     deserializer.deserialize_struct(
//       "BzTaskInfo",
//       &["src", "dest", "cache", "status"],
//       BzTaskInfoVisitor,
//     )
//   }
// }

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use super::*;

  #[test]
  fn test_serlize() {
    let task_info = BzTaskInfo {
      src: reqwest::Url::parse("https://svipsvip.ffzy-online5.com/20250118/37333_517b17a8/2000k/hls/mixed.m3u8").unwrap(),
      dest: PathBuf::from("./tmp"),
      cache: PathBuf::from("./tmp"),
      kind: BzTaskType::Zfs,
      status: BzTaskStatus::Queued,
    };
    let serialized = serde_json::to_string(&task_info).unwrap();
    println!("serialized = {}", serialized);

    let deserialized: BzTaskInfo = serde_json::from_str(&serialized).unwrap();
    println!("deserialized = {:?}", deserialized)
  }
}

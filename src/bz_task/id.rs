use std::fmt;
use std::hash::Hash;
use std::sync::atomic::{self, AtomicU64};

/// The id of the window.
#[derive(
  Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Deserialize,
)]
pub struct BzTaskId(u64);

static COUNT: AtomicU64 = AtomicU64::new(1);

impl BzTaskId {
  /// Creates a new unique window [`Id`].
  pub fn unique() -> BzTaskId {
    BzTaskId(COUNT.fetch_add(1, atomic::Ordering::Relaxed))
  }
}

impl fmt::Display for BzTaskId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.0.fmt(f)
  }
}

#[cfg(feature = "serialization")]
#[macro_use]
pub extern crate serde;
pub extern crate euclid;

mod allocator;
pub mod recording;

pub use crate::allocator::*;
pub use euclid::{size2, point2};

pub type Point = euclid::Point2D<i32>;
pub type Size = euclid::Size2D<i32>;
pub type Rectangle = euclid::Box2D<i32>;


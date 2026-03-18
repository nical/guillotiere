#![allow(mismatched_lifetime_syntaxes)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "serialization")]
#[macro_use]
pub extern crate serde;
pub extern crate euclid;

#[cfg(test)]
extern crate std;

mod allocator;
//pub mod recording;

pub use crate::allocator::*;
pub use euclid::{point2, size2};

pub type Point = euclid::default::Point2D<i32>;
pub type Size = euclid::default::Size2D<i32>;
pub type Rectangle = euclid::default::Box2D<i32>;

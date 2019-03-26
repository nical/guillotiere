
mod allocator;

pub use crate::allocator::{AtlasAllocator, AllocId};

pub struct DeviceSpace;
pub type DeviceIntRect = euclid::TypedRect<i32, DeviceSpace>;
pub type DeviceIntPoint = euclid::TypedPoint2D<i32, DeviceSpace>;
pub type DeviceIntSize = euclid::TypedSize2D<i32, DeviceSpace>;
pub use euclid::size2;


use std::ops::{Deref, Mul, MulAssign};

#[derive(Debug)]
pub struct LogicalPosition {
    pub x: f64,
    pub y: f64,
}

impl LogicalPosition {
    pub fn from_physical(position: &PhysicalPosition, scale: Scale) -> Self {
        LogicalPosition {
            x: position.x as f64 / *scale,
            y: position.y as f64 / *scale,
        }
    }

    pub fn to_physical(&self, scale: Scale) -> PhysicalPosition {
        PhysicalPosition::from_logical(self, scale)
    }
}

#[derive(Debug)]
pub struct PhysicalPosition {
    pub x: i32,
    pub y: i32,
}

impl PhysicalPosition {
    pub fn from_logical(position: &LogicalPosition, scale: Scale) -> Self {
        Self {
            x: (position.x * *scale) as i32,
            y: (position.y * *scale) as i32,
        }
    }

    pub fn to_logical(&self, scale: Scale) -> LogicalPosition {
        LogicalPosition::from_physical(self, scale)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LogicalSize {
    pub width: f64,
    pub height: f64,
}

impl LogicalSize {
    pub fn new(width: f64, height: f64) -> Self {
        LogicalSize { width, height }
    }

    pub fn from_physical(size: &PhysicalSize, scale: Scale) -> Self {
        Self {
            width: size.width as f64 / *scale,
            height: size.height as f64 / *scale,
        }
    }

    pub fn to_physical(&self, scale: Scale) -> PhysicalSize {
        PhysicalSize::from_logical(self, scale)
    }
}

impl Mul<Scale> for LogicalSize {
    type Output = LogicalSize;

    fn mul(self, rhs: Scale) -> Self::Output {
        LogicalSize {
            width: self.width * rhs.0,
            height: self.height * rhs.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PhysicalSize {
    pub width: u32,
    pub height: u32,
}

impl PhysicalSize {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
        }
    }

    pub fn from_logical(size: &LogicalSize, scale: Scale) -> Self {
        Self {
            width: (size.width * *scale).round() as u32,
            height: (size.height * *scale).round() as u32,
        }
    }

    pub fn to_logical(&self, scale: Scale) -> LogicalSize {
        LogicalSize::from_physical(self, scale)
    }
}

#[derive(Clone, Debug)]
pub struct Size {
    logical_size: LogicalSize,
    physical_size: PhysicalSize,
    scale: Scale,
}

impl Size {
    pub fn with_logical_size(logical_size: LogicalSize, scale: Scale) -> Self {
        Self {
            logical_size,
            physical_size: PhysicalSize::from_logical(&logical_size, scale),
            scale,
        }
    }

    pub fn with_physical_size(physical_size: PhysicalSize, scale: Scale) -> Self {
        Self {
            logical_size: LogicalSize::from_physical(&physical_size, scale),
            physical_size,
            scale,
        }
    }

    pub fn logical_size(&self) -> &LogicalSize {
        &self.logical_size
    }

    pub fn physical_size(&self) -> &PhysicalSize {
        &self.physical_size
    }

    pub fn scale(&self) -> Scale {
        self.scale
    }

    pub fn set_scale(&mut self, scale: Scale) {
        self.scale = scale;
        self.physical_size = PhysicalSize::from_logical(&self.logical_size, scale);
    }

    pub fn scale_by(&mut self, scale: Scale) {
        self.set_scale(self.scale * scale);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Scale(f64);

impl Default for Scale {
    fn default() -> Self {
        Self(1.0)
    }
}

impl Deref for Scale {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<f32> for Scale {
    fn from(value: f32) -> Self {
        Self(value as f64)
    }
}

impl From<f64> for Scale {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl Into<f64> for Scale {
    fn into(self) -> f64 {
        self.0
    }
}

impl Mul<f64> for Scale {
    type Output = Scale;

    fn mul(self, rhs: f64) -> Self::Output {
        Scale(self.0 * rhs)
    }
}

impl Mul<Scale> for Scale {
    type Output = Scale;

    fn mul(self, rhs: Scale) -> Self::Output {
        Scale(self.0 * rhs.0)
    }
}

impl MulAssign<f64> for Scale {
    fn mul_assign(&mut self, rhs: f64) {
        self.0 *= rhs;
    }
}

impl MulAssign<Scale> for Scale {
    fn mul_assign(&mut self, rhs: Scale) {
        self.0 *= rhs.0;
    }
}

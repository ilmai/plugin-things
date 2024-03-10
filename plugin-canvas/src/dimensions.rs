use std::ops::Mul;

#[derive(Clone, Debug)]
pub struct LogicalPosition {
    pub x: f64,
    pub y: f64,
}

impl LogicalPosition {
    pub fn from_physical(position: &PhysicalPosition, scale: f64) -> Self {
        LogicalPosition {
            x: position.x as f64 / scale,
            y: position.y as f64 / scale,
        }
    }

    pub fn to_physical(&self, scale: f64) -> PhysicalPosition {
        PhysicalPosition::from_logical(self, scale)
    }
}

impl<T> From<(T, T)> for LogicalPosition
where
    f64: From<T>,
{
    fn from(value: (T, T)) -> Self {
        Self {
            x: value.0.into(),
            y: value.1.into(),
        }
    }
}

#[derive(Debug)]
pub struct PhysicalPosition {
    pub x: i32,
    pub y: i32,
}

impl PhysicalPosition {
    pub fn from_logical(position: &LogicalPosition, scale: f64) -> Self {
        Self {
            x: (position.x * scale) as i32,
            y: (position.y * scale) as i32,
        }
    }

    pub fn to_logical(&self, scale: f64) -> LogicalPosition {
        LogicalPosition::from_physical(self, scale)
    }
}

impl<T> From<(T, T)> for PhysicalPosition
where
    i32: From<T>,
{
    fn from(value: (T, T)) -> Self {
        Self {
            x: value.0.into(),
            y: value.1.into(),
        }
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

    pub fn from_physical(size: &PhysicalSize, scale: f64) -> Self {
        Self {
            width: size.width as f64 / scale,
            height: size.height as f64 / scale,
        }
    }

    pub fn to_physical(&self, scale: f64) -> PhysicalSize {
        PhysicalSize::from_logical(self, scale)
    }
}

impl<T> From<(T, T)> for LogicalSize
where
    f64: From<T>,
{
    fn from(value: (T, T)) -> Self {
        Self {
            width: value.0.into(),
            height: value.1.into(),
        }
    }
}

impl Mul<f64> for LogicalSize {
    type Output = LogicalSize;

    fn mul(self, rhs: f64) -> Self::Output {
        LogicalSize {
            width: self.width * rhs,
            height: self.height * rhs,
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

    pub fn from_logical(size: &LogicalSize, scale: f64) -> Self {
        Self {
            width: (size.width * scale).round() as u32,
            height: (size.height * scale).round() as u32,
        }
    }

    pub fn to_logical(&self, scale: f64) -> LogicalSize {
        LogicalSize::from_physical(self, scale)
    }
}

impl<T> From<(T, T)> for PhysicalSize
where
    u32: From<T>,
{
    fn from(value: (T, T)) -> Self {
        Self {
            width: value.0.into(),
            height: value.1.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Size {
    logical_size: LogicalSize,
    physical_size: PhysicalSize,
    scale: f64,
}

impl Size {
    pub fn with_logical_size(logical_size: LogicalSize, scale: f64) -> Self {
        Self {
            logical_size,
            physical_size: PhysicalSize::from_logical(&logical_size, scale),
            scale,
        }
    }

    pub fn with_physical_size(physical_size: PhysicalSize, scale: f64) -> Self {
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

    pub fn scale(&self) -> f64 {
        self.scale
    }

    pub fn set_scale(&mut self, scale: f64) {
        self.scale = scale;
        self.physical_size = PhysicalSize::from_logical(&self.logical_size, scale);
    }

    pub fn scale_by(&mut self, scale: f64) {
        self.set_scale(self.scale * scale);
    }
}

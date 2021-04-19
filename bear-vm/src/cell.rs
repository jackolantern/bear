use std::mem::transmute_copy;
use std::convert::TryFrom;
use std::convert::TryInto;

/**
 * Represents a cell of memory.
 *
 * Size conversions, signed and unsigned operations, etc. are all here.
 */

pub type CellType = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cell(pub u32);
pub const SIZE: usize = std::mem::size_of::<u32>();

impl From<u32> for Cell { fn from(x: u32) -> Cell { Cell(x) } }
impl From<u16> for Cell { fn from(x: u16) -> Cell { Cell(x as u32) } }
impl From<u8> for Cell { fn from(x: u8) -> Cell { Cell(x as u32) } }
impl From<i32> for Cell { fn from(x: i32) -> Cell { Cell(unsafe { transmute_copy(&x) }) } }
impl From<i16> for Cell { fn from(x: i16) -> Cell { Cell::from(x as i32) } }
impl From<i8> for Cell { fn from(x: i8) -> Cell { Cell::from(x as i32) } }

impl Into<u32> for Cell {
    fn into(self) -> u32 {
        let Cell(x) = self;
        x
    }
}

impl Into<i32> for Cell {
    fn into(self) -> i32 {
        let Cell(x) = self;
        unsafe { transmute_copy(&x) }
    }
}

impl Into<usize> for Cell {
    fn into(self) -> usize {
        let Cell(x) = self;
        x as usize
    }
}

impl Into<isize> for Cell {
    fn into(self) -> isize {
        let Cell(x) = self;
        let x: i32 = unsafe { transmute_copy(&x) };
        x as isize
    }
}

impl TryInto<u8> for Cell {
    type Error = std::num::TryFromIntError;
    fn try_into(self) -> Result<u8, Self::Error> {
        let Cell(x) = self;
        x.try_into()
    }
}

impl TryInto<u16> for Cell {
    type Error = std::num::TryFromIntError;
    fn try_into(self) -> Result<u16, Self::Error> {
        let Cell(x) = self;
        x.try_into()
    }
}

impl TryInto<i8> for Cell {
    type Error = std::num::TryFromIntError;
    fn try_into(self) -> Result<i8, Self::Error> {
        let Cell(x) = self;
        x.try_into()
    }
}

impl TryInto<i16> for Cell {
    type Error = std::num::TryFromIntError;
    fn try_into(self) -> Result<i16, Self::Error> {
        let Cell(x) = self;
        x.try_into()
    }
}

impl TryFrom<isize> for Cell {
    type Error = std::num::TryFromIntError;
    fn try_from(x: isize) -> Result<Self, Self::Error> {
        let x: i32 = x.try_into()?;
        Ok(Cell::from(x))
    }
}

impl TryFrom<usize> for Cell {
    type Error = std::num::TryFromIntError;
    fn try_from(x: usize) -> Result<Self, Self::Error> {
        let x: u32 = x.try_into()?;
        Ok(Cell::from(x))
    }
}

impl std::ops::BitOr for Cell {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Cell(self.0 | other.0)
    }
}

impl std::ops::BitXor for Cell {
    type Output = Self;

    fn bitxor(self, other: Self) -> Self {
        Cell(self.0 ^ other.0)
    }
}

impl std::ops::BitAnd for Cell {
    type Output = Self;

    fn bitand(self, other: Self) -> Self {
        Cell(self.0 & other.0)
    }
}

impl std::ops::Not for Cell {
    type Output = Self;

    fn not(self) -> Self {
        Cell(!self.0)
    }
}

impl std::ops::Neg for Cell {
    type Output = Self;

    fn neg(self) -> Self {
        let x: i32 = unsafe { transmute_copy(&self.0) };
        let x = -x;
        let x: u32 = unsafe { transmute_copy(&x) };
        Cell(x)
    }
}

impl std::ops::Add for Cell {
    type Output = Self;

    fn add(self, other: Cell) -> Self {
        Cell(self.0.wrapping_add(other.0))
    }
}

impl std::ops::Sub for Cell {
    type Output = Self;

    fn sub(self, other: Cell) -> Self {
        Cell(self.0.wrapping_sub(other.0))
    }
}

impl std::ops::Div for Cell {
    type Output = Self;

    fn div(self, other: Cell) -> Self {
        Cell(self.0.wrapping_div(other.0))
    }
}

impl std::ops::Mul for Cell {
    type Output = Self;

    fn mul(self, other: Cell) -> Self {
        Cell(self.0.wrapping_mul(other.0))
    }
}

impl Cell {
    pub fn rem(self, other: Self) -> Cell {
        let r = self.0 % other.0;
        return r.into();
    }

    pub fn divmod(self, other: Self) -> (Cell, Cell) {
        let q = self.0 / other.0;
        let r = self.0 % other.0;
        return (q.into(), r.into());
    }
}

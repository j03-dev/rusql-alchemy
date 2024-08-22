#[cfg(feature = "postgres")]
pub type Serial = i32;

pub type Integer = i32;
pub type Text = String;
pub type Float = f64;
pub type Date = String;
pub type DateTime = String;
pub type Boolean = i32;

pub trait True {
    fn r#true() -> i32 {
        1
    }
}

pub trait False {
    fn r#false() -> i32 {
        0
    }
}

pub trait IsTrue {
    fn is_true(&self) -> bool;
}

impl True for Boolean {}

impl False for Boolean {}

impl IsTrue for Boolean {
    fn is_true(&self) -> bool {
        *self == 1
    }
}
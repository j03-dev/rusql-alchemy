#[cfg(feature = "postgres")]
pub type Serial = i32;

pub type Integer = i32;
pub type Text = String;
pub type Float = f64;
pub type Date = String;
pub type DateTime = String;
pub type Boolean = i32;

#[allow(non_upper_case_globals)]
pub const True: i32 = 1;
#[allow(non_upper_case_globals)]
pub const False: i32 = 0;

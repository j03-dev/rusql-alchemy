//! Query building blocks: [`Kwargs`](condition::Kwargs) conditions, the
//! SQL builders in [`builder`], and the [`SelectBuilder`](statement::SelectBuilder)
//! used by the `select!` macro.

pub mod builder;
pub mod condition;
pub mod statement;

pub struct Arg {
    pub value: String,
    pub ty: String,
}

#[derive(Default)]
pub struct Query {
    pub placeholders: String,
    pub fields: String,
    pub args: Vec<Arg>,
}

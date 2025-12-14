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

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

pub(crate) enum BindValue {
    Int32(Option<i32>),
    Int64(Option<i64>),
    Real(Option<f64>),
    Text(Option<String>),
}

impl BindValue {
    pub fn parse(value: &str, ty: &str) -> Result<Self, crate::Error> {
        let ty = ty.trim().trim_matches('"');
        let (inner, optional) = Self::inner_type(ty);
        let value = if optional && matches!(value, "null" | "None") {
            None
        } else {
            Some(value)
        };

        Ok(match inner {
            "i32" | "bool" | "i16" | "u8" | "u16" => Self::Int32(Self::parse_number(value)?),
            "i64" => Self::Int64(Self::parse_number(value)?),
            "f32" | "f64" => Self::Real(Self::parse_number(value)?),
            _ => Self::Text(value.map(Self::unquote_text)),
        })
    }

    fn parse_number<T>(value: Option<&str>) -> Result<Option<T>, crate::Error>
    where
        T: std::str::FromStr,
        crate::Error: From<T::Err>,
    {
        Ok(value
            .map(|v| v.trim_matches('"').parse::<T>())
            .transpose()?)
    }

    fn inner_type(ty: &str) -> (&str, bool) {
        const PREFIX: &str = "Option<";
        if let (Some(start), Some(end)) = (ty.find(PREFIX), ty.rfind('>')) {
            let inner_start = start + PREFIX.len();
            if end > inner_start {
                return (ty[inner_start..end].trim(), true);
            }
        }
        (ty, false)
    }

    fn unquote_text(value: &str) -> String {
        serde_json::from_str::<String>(value).unwrap_or_else(|_| value.trim_matches('"').to_owned())
    }
}

#[derive(Default)]
pub struct Query {
    pub placeholders: String,
    pub fields: String,
    pub args: Vec<Arg>,
}

#[cfg(test)]
mod tests {
    use super::BindValue;

    #[test]
    fn parses_plain_numeric_and_bool_types() {
        assert!(matches!(
            BindValue::parse("1", "i32").unwrap(),
            BindValue::Int32(Some(1))
        ));
        assert!(matches!(
            BindValue::parse("0", "bool").unwrap(),
            BindValue::Int32(Some(0))
        ));
        assert!(matches!(
            BindValue::parse("42", "i64").unwrap(),
            BindValue::Int64(Some(42))
        ));
        assert!(matches!(
            BindValue::parse("1.5", "f64").unwrap(),
            BindValue::Real(Some(1.5))
        ));
    }

    #[test]
    fn parses_qualified_option_types_with_and_without_values() {
        assert!(matches!(
            BindValue::parse("1", "core::option::Option<i32>").unwrap(),
            BindValue::Int32(Some(1))
        ));
        assert!(matches!(
            BindValue::parse("null", "core::option::Option<i32>").unwrap(),
            BindValue::Int32(None)
        ));
        assert!(matches!(
            BindValue::parse("null", "core::option::Option<bool>").unwrap(),
            BindValue::Int32(None)
        ));
        assert!(matches!(
            BindValue::parse("1.5", "core::option::Option<f64>").unwrap(),
            BindValue::Real(Some(1.5))
        ));
        assert!(matches!(
            BindValue::parse("null", "core::option::Option<f64>").unwrap(),
            BindValue::Real(None)
        ));
    }

    #[test]
    fn parses_optional_text_and_unescapes_quotes() {
        assert!(matches!(
            BindValue::parse("\"joe\"", "core::option::Option<alloc::string::String>").unwrap(),
            BindValue::Text(Some(ref s)) if s == "joe"
        ));
        assert!(matches!(
            BindValue::parse("null", "core::option::Option<alloc::string::String>").unwrap(),
            BindValue::Text(None)
        ));
        assert!(matches!(
            BindValue::parse("\"a\\\"b\"", "alloc::string::String").unwrap(),
            BindValue::Text(Some(ref s)) if s == "a\"b"
        ));
    }

    #[test]
    fn non_option_null_is_not_treated_as_a_null_bind() {
        assert!(BindValue::parse("null", "i32").is_err());
        assert!(matches!(
            BindValue::parse("null", "alloc::string::String").unwrap(),
            BindValue::Text(Some(ref s)) if s == "null"
        ));
    }

    #[test]
    fn invalid_numbers_are_reported() {
        assert!(BindValue::parse("nope", "i32").is_err());
        assert!(BindValue::parse("nope", "core::option::Option<f64>").is_err());
    }
}

use super::condition::Kwargs;
use super::{Arg, Query};
use crate::db::PLACEHOLDER;

pub fn to_update_query(kw: Vec<Kwargs>) -> Query {
    let mut args = Vec::new();
    let mut placeholders = Vec::new();
    let mut index = 0;
    for condition in kw {
        if let Kwargs::Condition {
            field,
            value,
            value_type,
            ..
        } = condition
        {
            index += 1;
            args.push(Arg {
                value,
                ty: value_type,
            });
            placeholders.push(format!("{field}={PLACEHOLDER}{index}",));
        }
    }

    Query {
        placeholders: placeholders.join(", "),
        args,
        ..Default::default()
    }
}

pub fn to_select_query(kw: Vec<Kwargs>) -> Query {
    let mut args = Vec::new();
    let mut placeholders = Vec::new();
    let mut index = 0;
    for condition in kw {
        match condition {
            Kwargs::Condition {
                field,
                value,
                value_type,
                comparison_operator,
            } => {
                if value_type == "column" {
                    placeholders.push(format!("{field}{comparison_operator}{value}"));
                } else {
                    index += 1;
                    args.push(Arg {
                        value,
                        ty: value_type,
                    });
                    placeholders.push(format!("{field}{comparison_operator}{PLACEHOLDER}{index}",));
                }
            }
            Kwargs::LogicalOperator { operator } => {
                placeholders.push(operator.to_owned());
            }
        }
    }

    Query {
        placeholders: placeholders.join(" "),
        args,
        ..Default::default()
    }
}

pub fn to_insert_query(kw: Vec<Kwargs>) -> Query {
    let mut args = Vec::new();
    let mut fields = Vec::new();
    let mut placeholders = Vec::new();
    let mut index = 0;
    for condition in kw {
        if let Kwargs::Condition {
            field,
            value,
            value_type,
            ..
        } = condition
        {
            index += 1;
            args.push(Arg {
                value,
                ty: value_type,
            });
            fields.push(field.clone());
            placeholders.push(format!("{PLACEHOLDER}{index}"));
        }
    }

    Query {
        placeholders: placeholders.join(", "),
        fields: fields.join(", "),
        args,
    }
}

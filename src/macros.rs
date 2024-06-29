#[macro_export]
macro_rules! kwargs {
    ($($field:ident = $value:expr),*) => {
        {
            let mut args = Vec::new();
            $(
                args.push(rusql_alchemy::db::models::Arg {
                    field: stringify!($field).to_string(),
                    value: rusql_alchemy::to_value($value.clone()),
                    r#type: rusql_alchemy::get_type_name($value.clone()).into(),
                    comparaison_operator: "=".to_string()
                });
            )*
            rusql_alchemy::db::models::Kwargs {
                operator: rusql_alchemy::db::models::Operator::And,
                args,
            }
        }
    };
    ($(Q!($field:ident = $value:expr)),*) => {
        {
            let mut args = Vec::new();
            $(
                args.push(Q!($field = $value)); 
            )*
            rusql_alchemy::db::models::Kwargs {
                operator: rusql_alchemy::db::models::Operator::And,
                args,
            }
        }
    };
}

#[macro_export]
macro_rules! Q {
    ($field:ident = $value:expr) => {
        match stringify!($field).split("__").collect::<Vec<&str>>()[..] {
            [field, operator] => {
                let comp_op = match operator {
                    "eq" => "=",
                    "ne" => "!=",
                    "gt" => ">",
                    "lt" => "<",
                    "ge" => ">=",
                    "le" => "<=",
                    wrong => panic!("comparaison operators is ['eq' 'ne' 'gt' 'lt' 'ge' 'le'] => {wrong} is not!")
                };

                rusql_alchemy::db::models::Arg {
                    field: field.to_string(),
                    value: rusql_alchemy::to_value($value.clone()),
                    r#type: rusql_alchemy::get_type_name($value.clone()).into(),
                    comparaison_operator: comp_op.to_string()
                }
            }
            _ => {
                panic!("Invalid field name");
            }
        }
    };
}

macro_rules! binds {
    ($args: expr, $stream:expr) => {
        for (t, v) in $args {
            match t.as_str() {
                "i32" | "bool" => {
                    $stream = $stream.bind(v.replace('"', "").parse::<i32>().unwrap());
                }
                "f64" => {
                    $stream = $stream.bind(v.replace('"', "").parse::<f64>().unwrap());
                }
                _ => {
                    $stream = $stream.bind(v.replace('"', ""));
                }
            }
        }
    };
}

#[macro_export]
macro_rules! migrate {
    ([$($struct:ident),*], $conn:expr) => {
        $( $struct::migrate($conn).await; )*
    };
}

#[macro_export]
macro_rules! kwargs {
    ($($key:ident = $value:expr),*) => {
        {
            let mut args = Vec::new();
            $(
                args.push(rusql_alchemy::db::models::Arg {
                    key: stringify!($key).to_string(),
                    value: rusql_alchemy::to_value($value.clone()),
                    r#type: rusql_alchemy::get_type_name($value.clone()).into()
                });
            )*
            rusql_alchemy::db::models::Kwargs {
                operator: rusql_alchemy::db::models::Operator::And,
                args,
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
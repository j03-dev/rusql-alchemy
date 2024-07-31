#[macro_export]
macro_rules! kwargs {
    // Support for direct field-value pairs with custom comparison operators
    ($($field:ident = $value:expr),* $(,)?) => {
        {
            let mut args = Vec::new();
            $(
                args.push(Condition::FieldCondition {
                    field: stringify!($field).to_string(),
                    value: rusql_alchemy::to_string($value.clone()),
                    value_type: rusql_alchemy::get_type_name($value.clone()).into(),
                    comparison_operator: "=".to_string(),
                });
            )*
            args
        }
    };
    ($field:ident == $value:expr) => {
        {
            vec![
                Condition::FieldCondition {
                    field: stringify!($field).to_string(),
                    value: rusql_alchemy::to_string($value.clone()),
                    value_type: rusql_alchemy::get_type_name($value.clone()).into(),
                    comparison_operator: "=".to_string(),
                }
            ]
        }
    };
    ($field:ident != $value:expr) => {
        {
            vec![
                Condition::FieldCondition {
                    field: stringify!($field).to_string(),
                    value: rusql_alchemy::to_string($value.clone()),
                    value_type: rusql_alchemy::get_type_name($value.clone()).into(),
                    comparison_operator: "!=".to_string(),
                }
            ]
        }
    };
    ($field:ident < $value:expr) => {
        {
            vec![
                Condition::FieldCondition {
                    field: stringify!($field).to_string(),
                    value: rusql_alchemy::to_string($value.clone()),
                    value_type: rusql_alchemy::get_type_name($value.clone()).into(),
                    comparison_operator: "<".to_string(),
                }
            ]
        }
    };
    ($field:ident <= $value:expr) => {
        {
            vec![
                Condition::FieldCondition {
                    field: stringify!($field).to_string(),
                    value: rusql_alchemy::to_string($value.clone()),
                    value_type: rusql_alchemy::get_type_name($value.clone()).into(),
                    comparison_operator: "<=".to_string(),
                }
            ]
        }
    };
    ($field:ident > $value:expr) => {
         {
             vec![
                 Condition::FieldCondition {
                    field: stringify!($field).to_string(),
                    value: rusql_alchemy::to_string($value.clone()),
                    value_type: rusql_alchemy::get_type_name($value.clone()).into(),
                    comparison_operator: ">".to_string(),
                 }
             ]
         }
     };
     ($field:ident >= $value:expr) => {
          {
              vec![
                  Condition::FieldCondition {
                    field: stringify!($field).to_string(),
                    value: rusql_alchemy::to_string($value.clone()),
                    value_type: rusql_alchemy::get_type_name($value.clone()).into(),
                    comparison_operator: ">=".to_string(),
                  }
              ]
          }
      };
}
macro_rules! binds {
    ($args: expr, $stream:expr) => {
        for (v, t) in $args {
            let v = v.replace('"', "");
            match t.as_str() {
                "i32" | "bool" => {
                    $stream = $stream.bind(v.parse::<i32>().unwrap());
                }
                "f64" => {
                    $stream = $stream.bind(v.parse::<f64>().unwrap());
                }
                _ => {
                    $stream = $stream.bind(v);
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

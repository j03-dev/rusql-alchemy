//! The condition types produced by the `kwargs!` macro.

#[derive(Debug, Clone)]
pub enum Kwargs {
    /// A single `field <operator> value` comparison.
    Condition {
        field: String,
        value: String,
        value_type: String,
        comparison_operator: String,
    },
    /// An `and` / `or` joining the conditions before and after it.
    LogicalOperator { operator: String },
}

/// Appends an `or` between `self` and `kwargs`.
pub trait Or {
    fn or(self, kwargs: Vec<Kwargs>) -> Vec<Kwargs>;
}

/// Appends an `and` between `self` and `kwargs`.
pub trait And {
    fn and(self, kwargs: Vec<Kwargs>) -> Vec<Kwargs>;
}

impl Or for Vec<Kwargs> {
    fn or(mut self, kwargs: Vec<Kwargs>) -> Vec<Kwargs> {
        self.push(Kwargs::LogicalOperator {
            operator: "or".to_string(),
        });
        self.extend(kwargs);
        self
    }
}

impl And for Vec<Kwargs> {
    fn and(mut self, kwargs: Vec<Kwargs>) -> Vec<Kwargs> {
        self.push(Kwargs::LogicalOperator {
            operator: "and".to_string(),
        });
        self.extend(kwargs);
        self
    }
}

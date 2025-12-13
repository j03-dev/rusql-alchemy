#[derive(Debug)]
pub enum Kwargs {
    Condition {
        field: String,
        value: String,
        value_type: String,
        comparison_operator: String,
    },
    LogicalOperator {
        operator: String,
    },
}

pub trait Or {
    fn or(self, kwargs: Vec<Kwargs>) -> Vec<Kwargs>;
}

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

use rusql_alchemy::prelude::*;

fn main() {
    let arg = Q!(user_id__eq = 0).or(Q!(user_id__ne = 1));
    // let kwarg = kwargs!(Q!(user_id__eq = 0), Q!(user_id__ne = 1)).or();
    // println!("{arg:#?}");
    // println!("{kwarg:#?}");
}

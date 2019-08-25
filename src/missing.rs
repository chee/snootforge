#[derive(Debug, PartialEq)]
pub enum Missing {
    Nowhere,
    // Sometime, // this come back when there are new things to implement
    Elsewhere(String),
}

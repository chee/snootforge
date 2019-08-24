#[derive(Debug, PartialEq)]
pub enum Missing {
    Nowhere,
    Sometime,
    Elsewhere(String),
}

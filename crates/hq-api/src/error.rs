#[derive(Debug)]
pub enum ApiError {
    Unauthorized,
    NotFound,
    Internal(String),
}

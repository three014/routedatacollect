pub mod api_interceptor;
pub mod cache;
pub mod google;
pub mod redis;

pub type GeneralResult = Result<(), Box<dyn std::error::Error>>;
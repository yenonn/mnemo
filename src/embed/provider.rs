use thiserror::Error;

#[derive(Error, Debug)]
pub enum EmbedError {
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Timeout")]
    Timeout,
}

pub trait EmbeddingProvider: Send + Sync {
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError>;
    fn dimensions(&self) -> usize;
}

pub struct StubProvider;

impl EmbeddingProvider for StubProvider {
    fn embed(&self, _text: &str) -> Result<Vec<f32>, EmbedError> {
        Ok(vec![0.0f32; 768])
    }
    fn dimensions(&self) -> usize {
        768
    }
}

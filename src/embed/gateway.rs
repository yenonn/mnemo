use super::provider::{EmbeddingProvider, EmbedError};

pub struct EmbeddingGateway {
    provider: Box<dyn EmbeddingProvider>,
}

impl EmbeddingGateway {
    pub fn new_default() -> Self {
        EmbeddingGateway {
            provider: Box::new(super::provider::StubProvider),
        }
    }
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        self.provider.embed(text)
    }
    pub fn dimensions(&self) -> usize {
        self.provider.dimensions()
    }
}

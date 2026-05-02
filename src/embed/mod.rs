pub mod provider;
pub mod gateway;

pub use provider::{EmbeddingProvider, EmbedError, StubProvider, OpenAiEmbeddingProvider, OllamaEmbeddingProvider};
pub use gateway::EmbeddingGateway;

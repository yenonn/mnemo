pub mod gateway;
pub mod provider;

pub use gateway::EmbeddingGateway;
pub use provider::{
    EmbedError, EmbeddingProvider, OllamaEmbeddingProvider, OpenAiEmbeddingProvider, StubProvider,
};

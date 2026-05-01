pub mod provider;
pub mod gateway;

pub use provider::{EmbeddingProvider, EmbedError, StubProvider};
pub use gateway::EmbeddingGateway;

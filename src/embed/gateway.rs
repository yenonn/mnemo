use super::provider::{
    EmbedError, EmbeddingProvider, OllamaEmbeddingProvider, OpenAiEmbeddingProvider, StubProvider,
};
use std::sync::OnceLock;

static GATEWAY_CACHE: OnceLock<Option<EmbeddingGateway>> = OnceLock::new();

pub struct EmbeddingGateway {
    provider: Box<dyn EmbeddingProvider>,
}

impl EmbeddingGateway {
    pub fn new_default() -> Self {
        EmbeddingGateway {
            provider: Box::new(StubProvider),
        }
    }

    /// Build from environment variables. Returns `None` when no provider is configured.
    pub fn from_env() -> Option<Self> {
        if let Ok(api_key) = std::env::var("MNEMO_OPENAI_API_KEY") {
            let model = std::env::var("MNEMO_OPENAI_MODEL")
                .unwrap_or_else(|_| "text-embedding-3-small".to_string());
            let dims: usize = std::env::var("MNEMO_EMBED_DIMS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1536);
            return Some(EmbeddingGateway {
                provider: Box::new(OpenAiEmbeddingProvider::new(&api_key, &model, dims)),
            });
        }

        if let Ok(endpoint) = std::env::var("MNEMO_OLLAMA_ENDPOINT") {
            let model = std::env::var("MNEMO_OLLAMA_MODEL")
                .unwrap_or_else(|_| "nomic-embed-text".to_string());
            let dims: usize = std::env::var("MNEMO_EMBED_DIMS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(768);
            return Some(EmbeddingGateway {
                provider: Box::new(OllamaEmbeddingProvider::new(&endpoint, &model, dims)),
            });
        }

        None
    }

    /// Build from environment variables, caching the result so that repeated
    /// calls do not re-read env vars or re-allocate the provider.
    pub fn from_env_cached() -> Option<&'static EmbeddingGateway> {
        GATEWAY_CACHE.get_or_init(|| Self::from_env()).as_ref()
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        self.provider.embed(text)
    }
    pub fn dimensions(&self) -> usize {
        self.provider.dimensions()
    }
}

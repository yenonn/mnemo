use thiserror::Error;
use serde_json::json;

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

/// OpenAI embedding provider (requires API key).
///
/// Uses `reqwest::blocking` for synchronous embedding in non-async contexts.
pub struct OpenAiEmbeddingProvider {
    api_key: String,
    model: String,
    dims: usize,
}

impl OpenAiEmbeddingProvider {
    pub fn new(api_key: &str, model: &str, dims: usize) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
            dims,
        }
    }
}

impl EmbeddingProvider for OpenAiEmbeddingProvider {
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        use reqwest::blocking::Client;

        let client = Client::new();
        let resp = client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "input": text,
            }))
            .send()
            .map_err(|e| EmbedError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EmbedError::Http(format!(
                "OpenAI returned {}",
                resp.status()
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .map_err(|e| EmbedError::InvalidResponse(e.to_string()))?;

        let data = json
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("embedding"))
            .and_then(|emb| emb.as_array())
            .ok_or_else(|| {
                EmbedError::InvalidResponse(
                    "Missing embedding data in OpenAI response".into(),
                )
            })?;

        let vec: Vec<f32> = data
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        if vec.len() != self.dims {
            return Err(EmbedError::InvalidResponse(format!(
                "Dimension mismatch: expected {}, got {}",
                self.dims,
                vec.len()
            )));
        }

        Ok(vec)
    }

    fn dimensions(&self) -> usize {
        self.dims
    }
}

/// Ollama embedding provider (local inference).
///
/// Calls a local Ollama instance via HTTP. No API key needed.
pub struct OllamaEmbeddingProvider {
    endpoint: String,
    model: String,
    dims: usize,
}

impl OllamaEmbeddingProvider {
    pub fn new(endpoint: &str, model: &str, dims: usize) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            model: model.to_string(),
            dims,
        }
    }
}

impl EmbeddingProvider for OllamaEmbeddingProvider {
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        use reqwest::blocking::Client;

        let client = Client::new();
        let resp = client
            .post(&self.endpoint)
            .json(&json!({
                "model": self.model,
                "prompt": text,
            }))
            .send()
            .map_err(|e| EmbedError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EmbedError::Http(format!(
                "Ollama returned {}",
                resp.status()
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .map_err(|e| EmbedError::InvalidResponse(e.to_string()))?;

        let arr = json
            .get("embedding")
            .and_then(|emb| emb.as_array())
            .ok_or_else(|| {
                EmbedError::InvalidResponse(
                    "Missing 'embedding' in Ollama response".into(),
                )
            })?;

        let vec: Vec<f32> = arr
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        if vec.len() != self.dims {
            return Err(EmbedError::InvalidResponse(format!(
                "Dimension mismatch: expected {}, got {}",
                self.dims,
                vec.len()
            )));
        }

        Ok(vec)
    }

    fn dimensions(&self) -> usize {
        self.dims
    }
}

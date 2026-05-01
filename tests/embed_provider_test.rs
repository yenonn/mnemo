use mnemo::embed::{EmbeddingProvider, StubProvider};

#[test]
fn test_stub_embed() {
    let provider = StubProvider;
    let result = provider.embed("test text").unwrap();
    assert_eq!(result.len(), 768); // dummy vector
    assert!(result.iter().all(|&v| v == 0.0));
}

#[test]
fn test_stub_dimensions() {
    let provider = StubProvider;
    assert_eq!(provider.dimensions(), 768);
}

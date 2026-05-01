use mnemo::embed::{EmbeddingGateway, EmbeddingProvider, StubProvider};

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

#[test]
fn test_gateway_new_default() {
    let gateway = EmbeddingGateway::new_default();
    assert_eq!(gateway.dimensions(), 768);
}

#[test]
fn test_gateway_embed() {
    let gateway = EmbeddingGateway::new_default();
    let result = gateway.embed("hello world").unwrap();
    assert_eq!(result.len(), 768);
    assert!(result.iter().all(|&v| v == 0.0));
}

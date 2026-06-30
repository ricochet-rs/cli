use mockito::Server;
use ricochet_cli::client::RicochetClient;
use ricochet_cli::config::ServerConfig;
use url::Url;

const TEST_PUB_PEM: &str = "-----BEGIN RSA PUBLIC KEY-----
MIIBCgKCAQEAr1XuDE4bFt7TnYqAtiRQ9RvC2sG3s8N8zUsCvhM+mZD7mGTN47bk
vYxKvp5ShVnM/6XZeCfRQA2TKXnf6dWsRgcZcBMufKHfN9VLNxawLMKHddceHlLA
rFTwsPE9rU9p5p5uA6zhUnZk/skzWumqZw9WK7Lztbh6fhX9UMYXvaBzCFF1nfTM
kGl7YkRcwfL4p+1oa7uGFYaRxvBKv6q9/hm7W9Em7H0g4+icc85wkvlzJrghKakp
5wDkaY8XmSGSiOZr0U8/fPBC4SASPuT5Hy17zZwu7SEYW31JYnRvFoo8bF8N3QxT
WigXLNxbQJjhAq7Y6mU8h7yF2zWMbFGMqwIDAQAB
-----END RSA PUBLIC KEY-----
";

#[tokio::test]
async fn fetches_and_parses_public_key() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("GET", "/api/v0/public-key")
        .with_status(200)
        .with_header("content-type", "application/x-pem-file")
        .with_body(TEST_PUB_PEM)
        .create();

    let cfg = ServerConfig {
        url: Url::parse(&server.url()).unwrap(),
        api_key: Some("test_api_key".to_string()),
    };
    let client = RicochetClient::new(&cfg).unwrap();

    let key = client.get_public_key().await;
    assert!(key.is_ok(), "expected a parsed key, got {key:?}");
}

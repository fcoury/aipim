# AI Primary Interface Module - AIPIM

AIPIM is a Rust library designed to provide a unified interface for interacting with various AI providers. It abstracts the complexities of different AI APIs, allowing developers to easily switch between providers without changing their codebase.

## Features

- Unified interface for multiple AI providers
- Support for text and image messages
- Asynchronous message sending
- Error handling and response parsing

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
aipim = "0.1.0"
```

## Usage

Here's a simple example to get you started:

```rust
use aipim::client::Client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let client = Client::new("gpt-4o");
    let response = client.message().text("Hello, world!").send().await?;
    println!("Response: {}", response.text);
    Ok(())
}
```

## Modules

- `client`: Contains the `Client` and `MessageBuilder` structs.
- `provider`: Contains the `AIProvider` trait and implementations for different providers.

## Contributing

We welcome contributions! Please see our [CONTRIBUTING.md](CONTRIBUTING.md) for more details.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

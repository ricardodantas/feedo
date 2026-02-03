# Async Rust with Tokio

## Overview

Tokio is the async runtime Feedo uses for non-blocking operations like fetching feeds.

## Key Concepts

### async/await Basics
```rust
// Async function
async fn fetch_data(url: &str) -> Result<String> {
    let response = reqwest::get(url).await?;
    let body = response.text().await?;
    Ok(body)
}

// Calling async functions
let data = fetch_data("https://example.com").await?;
```

### Tokio Runtime
```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Now you can use .await
    let result = some_async_function().await?;
    Ok(())
}
```

### Concurrent Fetching
```rust
use futures::future::join_all;

// Fetch multiple URLs in parallel
async fn fetch_all_feeds(urls: &[String]) -> Vec<Result<Feed>> {
    let futures: Vec<_> = urls.iter()
        .map(|url| fetch_feed(url))
        .collect();
    
    join_all(futures).await
}
```

### Timeouts
```rust
use tokio::time::{timeout, Duration};

let result = timeout(
    Duration::from_secs(30),
    fetch_feed(url)
).await;

match result {
    Ok(Ok(feed)) => println!("Got feed"),
    Ok(Err(e)) => println!("Fetch error: {e}"),
    Err(_) => println!("Timeout!"),
}
```

## Patterns Used in Feedo

### HTTP Client Setup
```rust
let client = reqwest::Client::builder()
    .user_agent("feedo/0.1.0")
    .timeout(Duration::from_secs(30))
    .build()?;
```

### Async Method in Sync Context
When you need to call async from a sync context (like input handlers):

```rust
// In App struct
pub async fn refresh_all(&mut self) {
    for i in 0..self.feeds.len() {
        self.refresh_feed(i).await;
    }
}

// Called from event loop (which is already async)
KeyCode::Char('r') => {
    self.refresh_all().await;
}
```

### Error Handling
```rust
async fn fetch_feed(&self, url: &str) -> Result<Vec<FeedItem>> {
    let response = self.client
        .get(url)
        .send()
        .await?;  // Network error
    
    let bytes = response
        .bytes()
        .await?;  // Read error
    
    let feed = parse_feed(&bytes)?;  // Parse error
    
    Ok(feed)
}
```

## Common Pitfalls

### Don't Block the Async Runtime
```rust
// BAD — blocks the entire runtime
std::thread::sleep(Duration::from_secs(1));

// GOOD — yields to other tasks
tokio::time::sleep(Duration::from_secs(1)).await;
```

### Don't Hold Locks Across Await
```rust
// BAD — can deadlock
let guard = mutex.lock().unwrap();
some_async_fn().await;  // Still holding lock!
drop(guard);

// GOOD — release lock before await
{
    let guard = mutex.lock().unwrap();
    // do sync stuff
}  // Lock released
some_async_fn().await;
```

### Use Arc for Shared State
```rust
use std::sync::Arc;
use tokio::sync::Mutex;

let shared_state = Arc::new(Mutex::new(AppState::default()));
let state_clone = Arc::clone(&shared_state);

tokio::spawn(async move {
    let mut state = state_clone.lock().await;
    state.update();
});
```

## Resources

- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Async Book](https://rust-lang.github.io/async-book/)

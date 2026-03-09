# Rust Error Handling

Production error handling in Rust using `thiserror` for typed library errors and `anyhow` for application-level error propagation.

## Core Concepts

### When to Use What

| Crate | Use case |
|-------|----------|
| `thiserror` | Library/crate APIs — define typed, structured errors |
| `anyhow` | Application code — ergonomic propagation with context |
| `std::error::Error` manually | When you need full control without derive macros |

### Error Categories

**Recoverable** (use `Result<T, E>`):
- Network timeouts, missing files, invalid input, rate limits

**Unrecoverable** (use `panic!`):
- Programming bugs, violated invariants, OOM in embedded

## Pattern 1: Typed Errors with `thiserror`

```rust
use thiserror::Error;

// Library-facing errors: structured, typed, derive Display automatically
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {field} — {message}")]
    InvalidInput { field: String, message: String },

    #[error("IO error")]
    Io(#[from] std::io::Error),           // auto From impl

    #[error("Parse error")]
    Parse(#[from] std::num::ParseIntError), // auto From impl

    #[error("Timeout after {0:?}")]
    Timeout(std::time::Duration),
}

pub type Result<T> = std::result::Result<T, AppError>;

// Usage — ? operator auto-converts via From
fn read_number_from_file(path: &str) -> Result<i32> {
    let contents = std::fs::read_to_string(path)?; // io::Error → AppError::Io
    let number = contents.trim().parse()?;          // ParseIntError → AppError::Parse
    Ok(number)
}
```

## Pattern 2: Layered Service Errors

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error("Unauthorized")]
    Unauthorized,
}

// Return typed errors from library/service functions
async fn get_user(id: &str) -> std::result::Result<User, ServiceError> {
    let result = db.query(id).await?; // sqlx::Error → ServiceError::Database

    match result {
        Some(user) => Ok(user),
        None => Err(ServiceError::NotFound(id.to_string())),
    }
}
```

## Pattern 3: Application Propagation with `anyhow`

```rust
use anyhow::{Context, Result, bail, ensure};

// Application code: ergonomic error propagation with rich context
async fn process_request(id: &str) -> Result<Response> {
    let data = fetch_data(id)
        .await
        .context("Failed to fetch data")?;           // adds context to any error

    let parsed = parse_response(&data)
        .with_context(|| format!("Failed to parse response for id={id}"))?;

    ensure!(!parsed.is_empty(), "Response was empty for id={id}"); // conditional bail

    Ok(parsed)
}

// bail! — return an error immediately
fn validate_age(age: i32) -> Result<()> {
    if age < 0 {
        bail!("Age must be non-negative, got {age}");
    }
    Ok(())
}

// Convert typed errors into anyhow at application boundary
async fn handle_request(id: &str) -> Result<()> {
    let user = get_user(id).await
        .context("Failed to fetch user")?; // ServiceError becomes anyhow::Error

    process_user(user).await
        .with_context(|| format!("Failed to process user {id}"))
}
```

## Pattern 4: Result and Option Combinators

```rust
use std::fs::File;
use std::io::{self, Read};

// Basic ? propagation
fn read_file(path: &str) -> Result<String, io::Error> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

// Option → Result conversion
fn find_user(id: &str) -> Option<User> {
    users.iter().find(|u| u.id == id).cloned()
}

fn get_user_age(id: &str) -> Result<u32, AppError> {
    find_user(id)
        .ok_or_else(|| AppError::NotFound(id.to_string()))
        .map(|user| user.age)
}

// Useful combinators
fn combinators_demo(result: Result<i32, AppError>) {
    // map: transform Ok value
    let doubled = result.map(|n| n * 2);

    // and_then: chain fallible operations
    let parsed: Result<i32, _> = "42".parse::<i32>()
        .map_err(AppError::Parse)
        .and_then(|n| if n > 0 { Ok(n) } else { Err(AppError::NotFound("negative".into())) });

    // unwrap_or / unwrap_or_else: provide fallbacks
    let value = result.unwrap_or(0);
    let value = result.unwrap_or_else(|e| {
        tracing::warn!("Using default due to: {e}");
        0
    });

    // ok(): convert Result<T, E> → Option<T> (drops error)
    let maybe: Option<i32> = result.ok();
}
```

## Pattern 5: Async Error Handling

```rust
use anyhow::{Context, Result};
use thiserror::Error;
use tokio::time::{timeout, Duration};

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Timeout after {0:?}")]
    Timeout(Duration),

    #[error("Service unavailable: {0}")]
    Unavailable(String),
}

// Timeout wrapper
async fn with_timeout<T, F>(duration: Duration, future: F) -> Result<T, ServiceError>
where
    F: std::future::Future<Output = Result<T, ServiceError>>,
{
    timeout(duration, future)
        .await
        .map_err(|_| ServiceError::Timeout(duration))?
}

// Retry with exponential backoff
async fn with_retry<T, F, Fut>(max_attempts: u32, mut f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut delay = Duration::from_millis(100);

    for attempt in 1..=max_attempts {
        match f().await {
            Ok(value) => return Ok(value),
            Err(e) if attempt < max_attempts => {
                tracing::warn!("Attempt {attempt} failed: {e:#}. Retrying in {delay:?}...");
                tokio::time::sleep(delay).await;
                delay *= 2; // exponential backoff
            }
            Err(e) => return Err(e),
        }
    }

    unreachable!()
}

// Usage
async fn fetch_with_retry(url: &str) -> Result<String> {
    with_retry(3, || async {
        reqwest::get(url)
            .await
            .context("HTTP request failed")?
            .text()
            .await
            .context("Failed to read response body")
    })
    .await
}
```

## Pattern 6: Panic Handling and Graceful Degradation

```rust
use std::panic;

// Catch panics from external/untrusted code
fn safe_call<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce() -> T + panic::UnwindSafe,
{
    panic::catch_unwind(f).map_err(|e| {
        if let Some(s) = e.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = e.downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic".to_string()
        }
    })
}

// Provide fallbacks — try primary, fall back on error
async fn get_data_with_fallback(id: &str) -> Result<Data> {
    match fetch_from_cache(id).await {
        Ok(data) => Ok(data),
        Err(e) => {
            tracing::warn!("Cache miss for {id}: {e}. Falling back to DB.");
            fetch_from_database(id).await
                .context("Both cache and DB failed")
        }
    }
}
```

## Error Reporting Patterns

```rust
// Use {:#} for full cause chain in logs (anyhow)
tracing::error!("Failed to process: {err:#}");

// Use {e} for user-facing messages (single level)
eprintln!("Error: {err}");

// In tests, use unwrap() or expect() with descriptive messages
let result = operation().expect("operation should succeed in test env");

// In main, report full chain before exiting
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if let Err(e) = run().await {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
    Ok(())
}
```

## Best Practices

### Do's
- **Use `thiserror`** for library/crate public error types
- **Use `anyhow`** for application-level propagation with `.context()`
- **Use `?` everywhere** — avoid manual `match` for propagation
- **Add context at boundaries** — `.context("what we were trying to do")?`
- **Log errors at the level that handles them** — not every propagation point

### Don'ts
- **Don't `.unwrap()` in production code** — use `?` or handle explicitly
- **Don't swallow errors silently** — at minimum log them
- **Don't lose context** — wrap errors with `.context()` when re-raising
- **Don't mix `anyhow` and `thiserror` at the same boundary** — pick one per layer
- **Don't `panic!` for expected failures** — panics are for bugs, not user errors

## Cargo.toml

```toml
[dependencies]
thiserror = "1.0"
anyhow = "1.0"
```

## References

- [thiserror crate](https://docs.rs/thiserror)
- [anyhow crate](https://docs.rs/anyhow)
- [Rust Book: Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)

# Rust Memory Safety Patterns

Ownership, borrowing, smart pointers, and concurrency primitives for memory-safe systems programming in Rust.

## Memory Bug Categories

| Bug Type | Description | Rust Prevention |
|----------|-------------|-----------------|
| **Use-after-free** | Access freed memory | Ownership — compiler rejects this |
| **Double-free** | Free same memory twice | Single owner — impossible by design |
| **Memory leak** | Never free memory | RAII — Drop runs automatically |
| **Buffer overflow** | Write past buffer end | Bounds-checked by default |
| **Dangling pointer** | Pointer to freed memory | Lifetime analysis at compile time |
| **Data race** | Concurrent unsynchronized access | Send/Sync traits enforced by compiler |

## Pattern 1: Ownership and Move Semantics

```rust
// Move semantics (default for heap types)
fn move_example() {
    let s1 = String::from("hello");
    let s2 = s1; // s1 is MOVED, no longer valid

    // println!("{}", s1); // Compile error!
    println!("{}", s2);
}

// Borrowing (references)
fn borrow_example() {
    let s = String::from("hello");

    // Immutable borrow — multiple allowed simultaneously
    let len = calculate_length(&s);
    println!("{} has length {}", s, len);

    // Mutable borrow — only one allowed at a time
    let mut s = String::from("hello");
    change(&mut s);
}

fn calculate_length(s: &String) -> usize {
    s.len()
} // s goes out of scope but doesn't drop (borrowed, not owned)

fn change(s: &mut String) {
    s.push_str(", world");
}
```

## Pattern 2: Lifetimes

```rust
// Compiler tracks reference validity via lifetimes
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}

// Struct holding a reference requires lifetime annotation
struct ImportantExcerpt<'a> {
    part: &'a str,
}

impl<'a> ImportantExcerpt<'a> {
    fn level(&self) -> i32 {
        3
    }

    // Lifetime elision: compiler infers 'a for &self return
    fn announce_and_return_part(&self, announcement: &str) -> &str {
        println!("Attention: {}", announcement);
        self.part
    }
}
```

## Pattern 3: Smart Pointers

```rust
// Box<T>: heap allocation, single owner
fn box_example() {
    let b = Box::new(5);
    println!("b = {}", b);
}

// Rc<T>: shared ownership, single-threaded
use std::rc::Rc;

fn rc_example() {
    let data = Rc::new(vec![1, 2, 3]);
    let data2 = Rc::clone(&data); // Increment reference count

    println!("Count: {}", Rc::strong_count(&data)); // 2
    // Freed when last Rc drops
}

// Arc<T>: shared ownership, thread-safe
use std::sync::Arc;
use std::thread;

fn arc_example() {
    let data = Arc::new(vec![1, 2, 3]);

    let handles: Vec<_> = (0..3)
        .map(|_| {
            let data = Arc::clone(&data);
            thread::spawn(move || {
                println!("{:?}", data);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}

// Weak<T>: break reference cycles
use std::rc::Weak;

struct Node {
    value: i32,
    children: Vec<Rc<Node>>,
    parent: Option<Weak<Node>>, // Weak prevents cycle
}
```

## Pattern 4: Interior Mutability

```rust
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// Cell<T>: for Copy types, no runtime borrow checking
struct Stats {
    count: Cell<i32>,
    data: RefCell<Vec<String>>, // RefCell for non-Copy types
}

impl Stats {
    fn increment(&self) {
        self.count.set(self.count.get() + 1);
    }

    fn add_data(&self, item: String) {
        self.data.borrow_mut().push(item); // panics on double-borrow
    }
}

// RefCell enforces borrow rules at runtime instead of compile time
// Use only when the borrow checker is overly restrictive and you
// can guarantee correct usage (e.g., the observer pattern)
```

## Pattern 5: Thread-Safe Concurrency

```rust
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;

// Atomic for simple numeric types (lock-free)
fn atomic_example() {
    let counter = Arc::new(AtomicI32::new(0));

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let counter = Arc::clone(&counter);
            thread::spawn(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    println!("Counter: {}", counter.load(Ordering::SeqCst));
}

// Mutex<T>: exclusive access to shared data
fn mutex_example() {
    let data = Arc::new(Mutex::new(vec![]));

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let data = Arc::clone(&data);
            thread::spawn(move || {
                let mut vec = data.lock().unwrap();
                vec.push(i);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}

// RwLock<T>: multiple readers OR one writer (read-heavy workloads)
fn rwlock_example() {
    use std::collections::HashMap;
    let data = Arc::new(RwLock::new(HashMap::<String, i32>::new()));

    // Multiple concurrent readers
    let read_guard = data.read().unwrap();
    drop(read_guard);

    // Exclusive writer
    let mut write_guard = data.write().unwrap();
    write_guard.insert("key".to_string(), 42);
}
```

## Pattern 6: RAII Resource Management

```rust
use std::fs::File;
use std::io::Write;

// Rust enforces RAII automatically via Drop
struct ManagedResource {
    file: File,
    name: String,
}

impl ManagedResource {
    fn new(path: &str) -> std::io::Result<Self> {
        let file = File::create(path)?;
        Ok(Self {
            file,
            name: path.to_string(),
        })
    }

    fn write(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.file.write_all(data)
    }
}

impl Drop for ManagedResource {
    fn drop(&mut self) {
        // Called automatically when ManagedResource goes out of scope
        println!("Releasing resource: {}", self.name);
        // file is also dropped here via its own Drop impl
    }
}

fn raii_example() -> std::io::Result<()> {
    let mut res = ManagedResource::new("output.txt")?;
    res.write(b"hello")?;
    // res.drop() called automatically here — file flushed and closed
    Ok(())
}
```

## Pattern 7: Bounds Checking

```rust
fn rust_bounds_checking() {
    let vec = vec![1, 2, 3, 4, 5];

    // Runtime bounds check (panics if out of bounds)
    let val = vec[2];

    // Explicit safe access returning Option
    match vec.get(10) {
        Some(val) => println!("Got {}", val),
        None => println!("Index out of bounds"),
    }

    // Iterators — no bounds checking needed, always safe
    for val in &vec {
        println!("{}", val);
    }

    // Slices are bounds-checked
    let slice = &vec[1..3]; // [2, 3]
}
```

## Best Practices

### Do's
- **Prefer ownership** — Pass by value when the callee should own data
- **Use `&` for read-only access** — Borrow instead of cloning
- **Reach for `Arc<Mutex<T>>`** — For shared mutable state across threads
- **Use `AtomicT`** — For simple counters/flags (lock-free)
- **Minimize `unsafe`** — Encapsulate and document invariants carefully

### Don'ts
- **Don't clone to satisfy the borrow checker** — Understand the real ownership need
- **Don't hold `Mutex` guards across `.await`** — Use `tokio::sync::Mutex` in async code
- **Don't use `RefCell` in multithreaded code** — Use `Mutex` instead
- **Don't ignore compiler warnings** — They catch real bugs
- **Don't use `unsafe` carelessly** — Every `unsafe` block requires proof of soundness

## Debugging Tools

```bash
# Miri: undefined behavior detector for Rust
cargo +nightly miri run

# AddressSanitizer
RUSTFLAGS="-Z sanitizer=address" cargo +nightly run

# ThreadSanitizer
RUSTFLAGS="-Z sanitizer=thread" cargo +nightly run

# Loom: systematic concurrency testing
# Add to dev-dependencies: loom = "0.7"
```

## References

- [Rust Ownership](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [Rustonomicon (Unsafe Rust)](https://doc.rust-lang.org/nomicon/)
- [Miri](https://github.com/rust-lang/miri)

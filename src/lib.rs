//! Showcase: Universe-parameterized HTTP client using associated traits.
//!
//! Demonstrates `#![feature(associated_traits)]` — a single `Universe` trait
//! abstracts over thread-safety constraints so the same generic types work
//! with both `Arc`/`Mutex` (shared) and `Rc`/`RefCell` (isolated).

#![feature(associated_traits)]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Helper traits for smart pointers and interior-mutability cells
// ---------------------------------------------------------------------------

pub trait RefLike<T>: Clone {
    fn new(val: T) -> Self;
}

pub trait CellLike<T> {
    fn new(val: T) -> Self;
}

impl<T> RefLike<T> for Arc<T> {
    fn new(val: T) -> Self {
        Arc::new(val)
    }
}

impl<T> RefLike<T> for Rc<T> {
    fn new(val: T) -> Self {
        Rc::new(val)
    }
}

impl<T> CellLike<T> for Mutex<T> {
    fn new(val: T) -> Self {
        Mutex::new(val)
    }
}

impl<T> CellLike<T> for RefCell<T> {
    fn new(val: T) -> Self {
        RefCell::new(val)
    }
}

// ---------------------------------------------------------------------------
// The Universe trait — the core abstraction using associated traits
// ---------------------------------------------------------------------------

/// A "universe" determines the threading model for all generic infrastructure.
///
/// * **`trait Bounds`** is an *associated trait* — the new language feature.
///   It lets each universe declare what constraints inner values must satisfy.
/// * **`Ref`** and **`Cell`** are GATs bounded by `Self::Bounds`, so the
///   compiler enforces that `Arc`/`Mutex` only appear in the `Send + Sync`
///   universe and `Rc`/`RefCell` only in the isolated one.
pub trait Universe {
    trait Bounds;
    type Ref<T: Self::Bounds>: RefLike<T> + Self::Bounds;
    type Cell<T: Self::Bounds>: CellLike<T> + Self::Bounds;
}

// ---------------------------------------------------------------------------
// Shared universe — thread-safe (Arc + Mutex)
// ---------------------------------------------------------------------------

pub struct Shared;

impl Universe for Shared {
    trait Bounds = Send + Sync + 'static;
    type Ref<T: Self::Bounds> = Arc<T>;
    type Cell<T: Self::Bounds> = Mutex<T>;
}

// ---------------------------------------------------------------------------
// Isolated universe — single-thread (Rc + RefCell)
// ---------------------------------------------------------------------------

pub struct Isolated;

impl Universe for Isolated {
    trait Bounds = 'static;
    type Ref<T: Self::Bounds> = Rc<T>;
    type Cell<T: Self::Bounds> = RefCell<T>;
}

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

pub struct State {
    pub base_url: String,
    pub request_count: u64,
}

/// An HTTP client generic over the threading universe.
///
/// * `HttpClient<Shared>`  → holds `Arc<Mutex<State>>`, is `Send`.
/// * `HttpClient<Isolated>` → holds `Rc<RefCell<State>>`, is `!Send`.
pub struct HttpClient<U: Universe>
{
    state: U::Ref<U::Cell<State>>,
}

impl<U: Universe> HttpClient<U>
{
    pub fn new(base_url: String) -> Self {
        let state = State { base_url, request_count: 0 };
        HttpClient {
            state: RefLike::new(CellLike::new(state)),
        }
    }

    pub fn get(&self, _path: &str) -> String {
        // Stub — real implementation would use the state
        String::from("200 OK")
    }

    pub fn clone_ref(&self) -> Self {
        HttpClient { state: self.state.clone() }
    }
}

/// Application service showcasing how the universe cascades from app-level
/// types down into library types.
pub struct AppService<U: Universe>
{
    client: HttpClient<U>,
}

impl<U: Universe> AppService<U>
{
    pub fn new(base_url: String) -> Self {
        AppService {
            client: HttpClient::new(base_url),
        }
    }

    pub fn fetch_data(&self) -> String {
        self.client.get("/data")
    }
}

// ---------------------------------------------------------------------------
// Compile-time proofs
// ---------------------------------------------------------------------------

// HttpClient<Shared> is Send + Sync (because Arc<Mutex<State>> is).
const _: () = {
    fn assert_send_sync<T: Send + Sync>() {}
    fn check() {
        assert_send_sync::<HttpClient<Shared>>();
        assert_send_sync::<AppService<Shared>>();
    }
};

// HttpClient<Isolated> is !Send (because Rc<RefCell<State>> is !Send).
// Uncomment the following to see a compile error proving !Send:
//
//   const _: () = {
//       fn assert_send<T: Send>() {}
//       fn check() { assert_send::<HttpClient<Isolated>>(); }
//   };

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_client() {
        let svc = AppService::<Shared>::new("https://api.example.com".into());
        assert_eq!(svc.fetch_data(), "200 OK");
    }

    #[test]
    fn isolated_client() {
        let svc = AppService::<Isolated>::new("https://api.example.com".into());
        assert_eq!(svc.fetch_data(), "200 OK");
    }
}

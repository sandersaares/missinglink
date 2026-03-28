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
/// Two *associated traits* (the new language feature) separate input
/// requirements from output guarantees:
///
/// * **`BoundsIn`** — what inner values must satisfy to enter the universe.
///   For `Shared` this is `Send + 'static` (not `Sync` — `Mutex` provides that).
/// * **`BoundsOut`** — what the universe's wrapper types guarantee.
///   For `Shared` this is `Send + Sync + 'static`.
///
/// `Cell` bridges the two: it accepts `BoundsIn` and its output satisfies
/// `BoundsOut`. This models how `Mutex<T: Send>` is itself `Send + Sync`.
pub trait Universe {
    trait BoundsIn;
    trait BoundsOut;
    type Ref<T: Self::BoundsOut>: RefLike<T> + Self::BoundsOut;
    type Cell<T: Self::BoundsIn>: CellLike<T> + Self::BoundsOut;
}

// ---------------------------------------------------------------------------
// Shared universe — thread-safe (Arc + Mutex)
// ---------------------------------------------------------------------------

pub struct Shared;

impl Universe for Shared {
    trait BoundsIn = Send + 'static;
    trait BoundsOut = Send + Sync + 'static;
    type Ref<T: Self::BoundsOut> = Arc<T>;
    type Cell<T: Self::BoundsIn> = Mutex<T>;
}

// ---------------------------------------------------------------------------
// Isolated universe — single-thread (Rc + RefCell)
// ---------------------------------------------------------------------------

pub struct Isolated;

impl Universe for Isolated {
    trait BoundsIn = 'static;
    trait BoundsOut = 'static;
    type Ref<T: Self::BoundsOut> = Rc<T>;
    type Cell<T: Self::BoundsIn> = RefCell<T>;
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

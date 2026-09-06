//! Tenet application use cases and infrastructure ports.
//!
//! Concrete repository, persistence, and process execution implementations are
//! supplied by adapters implementing [`ports`] over the domain kernel in
//! [`tenet_kernel`].

pub mod application;
pub mod ports;
pub mod response;

mod auth;
mod handlers;
mod server;

#[cfg(not(feature = "custom-protocol"))]
pub use server::start_bridge;

#[cfg(feature = "custom-protocol")]
pub use server::start_bridge_with_app;

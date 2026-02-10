//! Message bus module for decoupled channel-agent communication.
//!
//! This module provides an async message bus that enables communication
//! between chat channels and the agent core using message passing patterns.

pub mod events;
pub mod queue;

pub use events::{InboundMessage, OutboundMessage};
pub use queue::MessageBus;

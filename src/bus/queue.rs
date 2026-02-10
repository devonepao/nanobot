//! Async message queue for decoupled channel-agent communication.

use crate::bus::events::{InboundMessage, OutboundMessage};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{timeout, Duration};
use tracing::{error, trace};

type OutboundCallback = Arc<dyn Fn(OutboundMessage) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync>;

/// Async message bus that decouples chat channels from the agent core.
///
/// Channels push messages to the inbound queue, and the agent processes
/// them and pushes responses to the outbound queue.
pub struct MessageBus {
    inbound_tx: mpsc::UnboundedSender<InboundMessage>,
    inbound_rx: Arc<Mutex<mpsc::UnboundedReceiver<InboundMessage>>>,
    outbound_tx: mpsc::UnboundedSender<OutboundMessage>,
    outbound_rx: Arc<Mutex<mpsc::UnboundedReceiver<OutboundMessage>>>,
    outbound_subscribers: Arc<Mutex<HashMap<String, Vec<OutboundCallback>>>>,
    running: Arc<Mutex<bool>>,
}

impl MessageBus {
    /// Create a new message bus
    pub fn new() -> Self {
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();

        Self {
            inbound_tx,
            inbound_rx: Arc::new(Mutex::new(inbound_rx)),
            outbound_tx,
            outbound_rx: Arc::new(Mutex::new(outbound_rx)),
            outbound_subscribers: Arc::new(Mutex::new(HashMap::new())),
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// Publish a message from a channel to the agent.
    pub async fn publish_inbound(&self, msg: InboundMessage) -> Result<(), String> {
        self.inbound_tx
            .send(msg)
            .map_err(|e| format!("Failed to publish inbound message: {}", e))
    }

    /// Consume the next inbound message (blocks until available).
    pub async fn consume_inbound(&self) -> Option<InboundMessage> {
        let mut rx = self.inbound_rx.lock().await;
        rx.recv().await
    }

    /// Publish a response from the agent to channels.
    pub async fn publish_outbound(&self, msg: OutboundMessage) -> Result<(), String> {
        self.outbound_tx
            .send(msg)
            .map_err(|e| format!("Failed to publish outbound message: {}", e))
    }

    /// Consume the next outbound message (blocks until available).
    pub async fn consume_outbound(&self) -> Option<OutboundMessage> {
        let mut rx = self.outbound_rx.lock().await;
        rx.recv().await
    }

    /// Subscribe to outbound messages for a specific channel.
    pub async fn subscribe_outbound<F, Fut>(&self, channel: String, callback: F)
    where
        F: Fn(OutboundMessage) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let callback_boxed: OutboundCallback = Arc::new(move |msg| {
            Box::pin(callback(msg))
        });

        let mut subscribers = self.outbound_subscribers.lock().await;
        subscribers
            .entry(channel)
            .or_insert_with(Vec::new)
            .push(callback_boxed);
    }

    /// Dispatch outbound messages to subscribed channels.
    /// Run this as a background task.
    pub async fn dispatch_outbound(&self) {
        *self.running.lock().await = true;

        while *self.running.lock().await {
            let msg_result = timeout(Duration::from_secs(1), async {
                let mut rx = self.outbound_rx.lock().await;
                rx.recv().await
            })
            .await;

            match msg_result {
                Ok(Some(msg)) => {
                    let subscribers = self.outbound_subscribers.lock().await;
                    if let Some(callbacks) = subscribers.get(&msg.channel) {
                        for callback in callbacks {
                            let msg_clone = msg.clone();
                            let callback_clone = Arc::clone(callback);
                            tokio::spawn(async move {
                                callback_clone(msg_clone).await;
                            });
                        }
                    }
                }
                Ok(None) => {
                    trace!("Outbound channel closed");
                    break;
                }
                Err(_) => {
                    // Timeout, continue loop
                    continue;
                }
            }
        }
    }

    /// Stop the dispatcher loop.
    pub async fn stop(&self) {
        *self.running.lock().await = false;
    }

    /// Number of pending inbound messages.
    ///
    /// Note: This method always returns 0 as tokio's mpsc channels do not expose
    /// queue size. This method is provided for API compatibility with the Python version.
    /// Consider removing calls to this method if queue size monitoring is needed.
    pub fn inbound_size(&self) -> usize {
        0
    }

    /// Number of pending outbound messages.
    ///
    /// Note: This method always returns 0 as tokio's mpsc channels do not expose
    /// queue size. This method is provided for API compatibility with the Python version.
    /// Consider removing calls to this method if queue size monitoring is needed.
    pub fn outbound_size(&self) -> usize {
        0
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_inbound_message_flow() {
        let bus = MessageBus::new();
        let msg = InboundMessage::new(
            "test".to_string(),
            "user1".to_string(),
            "chat1".to_string(),
            "Hello".to_string(),
        );

        bus.publish_inbound(msg.clone()).await.unwrap();
        let received = bus.consume_inbound().await.unwrap();

        assert_eq!(received.channel, "test");
        assert_eq!(received.sender_id, "user1");
        assert_eq!(received.content, "Hello");
    }

    #[tokio::test]
    async fn test_outbound_message_flow() {
        let bus = MessageBus::new();
        let msg = OutboundMessage::new(
            "test".to_string(),
            "chat1".to_string(),
            "Hello back".to_string(),
        );

        bus.publish_outbound(msg.clone()).await.unwrap();
        let received = bus.consume_outbound().await.unwrap();

        assert_eq!(received.channel, "test");
        assert_eq!(received.content, "Hello back");
    }

    #[tokio::test]
    async fn test_subscribe_and_dispatch() {
        let bus = Arc::new(MessageBus::new());
        let received_msg = Arc::new(Mutex::new(None));
        let received_clone = Arc::clone(&received_msg);

        bus.subscribe_outbound("test".to_string(), move |msg| {
            let received = Arc::clone(&received_clone);
            async move {
                *received.lock().await = Some(msg);
            }
        })
        .await;

        let bus_clone = Arc::clone(&bus);
        tokio::spawn(async move {
            bus_clone.dispatch_outbound().await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let msg = OutboundMessage::new(
            "test".to_string(),
            "chat1".to_string(),
            "Dispatched".to_string(),
        );

        bus.publish_outbound(msg).await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;

        let received = received_msg.lock().await;
        assert!(received.is_some());
        assert_eq!(received.as_ref().unwrap().content, "Dispatched");

        bus.stop().await;
    }
}

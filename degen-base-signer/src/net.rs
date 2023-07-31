use serde::{Deserialize, Serialize};
use std::{fmt::Debug, time::Duration};
use tracing::{debug, warn};

use crate::signing_round;
// Message is the format over the wire
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub msg: signing_round::MessageTypes,
    pub sig: Vec<u8>,
}

// Http listen/poll with queue (requires mutable access, is configured by passing in HttpNet)
#[derive(Clone)]
pub struct HttpNetListen {
    pub net: HttpNet,
    in_queue: Vec<Message>,
}

impl HttpNetListen {
    pub fn new(net: HttpNet, in_queue: Vec<Message>) -> Self {
        HttpNetListen { net, in_queue }
    }
}

// Http send (does not require mutable access, can be cloned to pass to threads)
#[derive(Clone)]
pub struct HttpNet {
    pub http_relay_url: String,
    connected: bool,
}

impl HttpNet {
    pub fn new(http_relay_url: String) -> Self {
        HttpNet {
            http_relay_url,
            connected: true,
        }
    }
}

// these functions manipulate the inbound message queue
pub trait NetListen {
    type Error: Debug;

    fn listen(&self);
    fn poll(&mut self, id: u32);
    fn next_message(&mut self) -> Option<Message>;
    fn send_message(&self, msg: Message) -> Result<(), Self::Error>;
}

impl NetListen for HttpNetListen {
    type Error = Error;

    fn listen(&self) {}

    fn poll(&mut self, id: u32) {
        let url = url_with_id(&self.net.http_relay_url, id);
        debug!("poll {}", url);
        match ureq::get(&url).call() {
            Ok(response) => {
                self.net.connected = true;
                if response.status() == 200 {
                    match bincode::deserialize_from::<_, Message>(response.into_reader()) {
                        Ok(msg) => {
                            debug!("received {:?}", msg);
                            self.in_queue.push(msg);
                        }
                        Err(_e) => {}
                    };
                };
            }
            Err(e) => {
                if self.net.connected {
                    warn!("{} U: {}", e, url);
                    self.net.connected = false;
                }
            }
        };
    }
    fn next_message(&mut self) -> Option<Message> {
        self.in_queue.pop()
    }

    // pass-thru to immutable net function
    fn send_message(&self, msg: Message) -> Result<(), Self::Error> {
        self.net.send_message(msg)
    }
}

// for threads that only send data, use immutable Net
pub trait Net {
    type Error: Debug;

    fn send_message(&self, msg: Message) -> Result<(), Self::Error>;
}

impl Net for HttpNet {
    type Error = Error;

    fn send_message(&self, msg: Message) -> Result<(), Self::Error> {
        // sign message
        let bytes = bincode::serialize(&msg)?;

        let notify = |_err, dur| {
            debug!(
                "Failed to connect to {}. Next attempt in {:?}",
                &self.http_relay_url, dur
            );
        };

        let send_request = || {
            ureq::post(&self.http_relay_url)
                .send_bytes(&bytes[..])
                .map_err(backoff::Error::transient)
        };
        let backoff_timer = backoff::ExponentialBackoffBuilder::new()
            .with_initial_interval(Duration::from_millis(2))
            .with_max_interval(Duration::from_millis(128))
            .build();

        let response = backoff::retry_notify(backoff_timer, send_request, notify)
            .map_err(|_| Error::Timeout)?;

        debug!(
            "sent {:?} {} bytes {:?} to {}",
            &msg.msg,
            bytes.len(),
            &response,
            self.http_relay_url
        );
        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Serialization failed: {0}")]
    SerializationError(#[from] bincode::Error),
    #[error("{0}")]
    NetworkError(#[from] Box<ureq::Error>),
    #[error("Failed to connect to network.")]
    Timeout,
}

fn url_with_id(base: &str, id: u32) -> String {
    let mut url = base.to_owned();
    url.push_str(&format!("?id={id}"));
    url
}

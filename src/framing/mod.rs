use wtransport::{RecvStream, SendStream};

// TODO: make two submodules, use two approaches:
// - implement trait to take asynchronous callback and stream, and call the calback for each frame
// received after parsing
// - check implementation of tungstenite-rs and try to implement same

pub trait Framing {
}

use std::{error::Error, future::Future};
use wtransport::{RecvStream, SendStream};

// TODO: make two submodules, use two approaches:
// - implement trait to take asynchronous callback and stream, and call the calback for each frame
// received after parsing
// - check implementation of tungstenite-rs and try to implement same

pub struct FrameHeader {
	is_final: bool,
	payload_length: u64,
}

pub struct Frame {
	header: FrameHeader,
	content: Box<[u8]>,
}

impl Frame {
	#[inline]
	pub fn header(&self) -> &FrameHeader {
		&self.header
	}
	pub fn get_content(self) -> Box<[u8]> {
		self.content
	}
}

impl FrameHeader {
	#[inline]
	pub fn is_final(&self) -> bool {
		self.is_final
	}
	#[inline]
	pub fn payload_length(&self) -> u64 {
		self.payload_length
	}
}

pub trait FrameReader {
	type Error;
	fn read_frame(
		&mut self,
	) -> impl Future<Output = Result<Frame, Self::Error>> + Send;
}

pub trait FrameWriter {
	type Error: Error;
	fn write_frame(
		&mut self,
		frame: Frame,
	) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

pub trait MessageReader {
	type Error;
	const MAX_MESSAGE_SIZE_BYTES: usize = 65535;
	fn read_message(
		&mut self,
	) -> impl Future<Output = Result<Vec<u8>, Self::Error>> + Send;
}

pub trait MessageWriter {
	type Error;
	const MAX_CHUNK_SIZE_BYTES: usize = 1024;
	fn write_message(
		&mut self,
		buffer: impl AsRef<[u8]> + Send,
	) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

impl FrameReader for RecvStream {
	type Error = wtransport::error::StreamReadExactError;
	#[inline]
	async fn read_frame(&mut self) -> Result<Frame, Self::Error> {
		let mut header_bytes = [0];
		self.read_exact(&mut header_bytes).await?;

		static FIN_BIT_MASK: u8 = 0b1000_0000;
		static PAYLOAD_LENGTH_MASK: u8 = 0b0111_1111;

		let is_final = header_bytes[0] & FIN_BIT_MASK != 0;
		let payload_length_bytes = header_bytes[0] & PAYLOAD_LENGTH_MASK;

		let payload_length = match payload_length_bytes {
			126 => {
				let mut payload_length_bytes = [0; 2];
				self.read_exact(&mut payload_length_bytes).await?;
				u16::from_be_bytes(payload_length_bytes) as u64
			}
			127 => {
				let mut payload_length_bytes = [0; 8];
				self.read_exact(&mut payload_length_bytes).await?;
				u64::from_be_bytes(payload_length_bytes)
			}
			_ => payload_length_bytes as u64,
		};

		let header = FrameHeader {
			is_final,
			payload_length,
		};

		let mut content = vec![0; payload_length as usize].into_boxed_slice();
		self.read_exact(&mut content).await?;

		Ok(Frame { content, header })
	}
}

impl FrameWriter for SendStream {
	type Error = wtransport::error::StreamWriteError;
	async fn write_frame(&mut self, frame: Frame) -> Result<(), Self::Error> {
		let buffer = &frame.content;
		let payload_length = frame.header().payload_length;

		let (extra_bytes, payload_length_length): (u8, u8) =
			match payload_length {
				0..=125 => (0, payload_length as u8),
				126..=65535 => (2, 126),
				_ => (8, 127),
			};

		let header_byte = if frame.header().is_final() {
			128u8
		} else {
			0u8
		} | payload_length_length;
		self.write_all(&[header_byte]).await?;
		if extra_bytes == 2 {
			self.write_all(&(payload_length as u16).to_be_bytes())
				.await?;
		} else if extra_bytes == 8 {
			self.write_all(&payload_length.to_be_bytes()).await?;
		}
		self.write_all(buffer).await?;
		Ok(())
	}
}

#[derive(Debug, thiserror::Error)]
pub enum MessageReadError {
	#[error(transparent)]
	TransportError(wtransport::error::StreamReadExactError),
	#[error("message size exceeded max message size limit")]
	MessageSizeExceeded,
}

impl From<wtransport::error::StreamReadExactError> for MessageReadError {
	fn from(value: wtransport::error::StreamReadExactError) -> Self {
		Self::TransportError(value)
	}
}

impl MessageReader for RecvStream {
	// FIXME: this implementation can be much much faster if we directly parse from stream instead
	// of making frames and repeatedly copying all data from here to there.
	type Error = MessageReadError;
	async fn read_message(&mut self) -> Result<Vec<u8>, Self::Error> {
		let mut message = Vec::new();
		let mut current_len = 0;
		loop {
			let frame = self.read_frame().await?;
			let is_final = frame.header().is_final();
			let content = frame.get_content();
			// TODO: check overflow when adding const generics to function
			current_len += content.len();
			if current_len > Self::MAX_MESSAGE_SIZE_BYTES {
				return Err(MessageReadError::MessageSizeExceeded);
			}
			message.extend_from_slice(&content);
			if is_final {
				return Ok(message);
			}
		}
	}
}

impl MessageWriter for SendStream {
	type Error = wtransport::error::StreamWriteError;
	async fn write_message(
		&mut self,
		buffer: impl AsRef<[u8]>,
	) -> Result<(), Self::Error> {
		let buffer = buffer.as_ref();
		let last = (buffer.len() as f64 / Self::MAX_CHUNK_SIZE_BYTES as f64)
			.ceil() as usize
			- 1;
		let chunks_enumerated = buffer
			.chunks(Self::MAX_CHUNK_SIZE_BYTES)
			.map(|c| c.to_owned().into_boxed_slice())
			.enumerate()
			.map(|(idx, c)| (idx == last, c));

		for (is_final, content) in chunks_enumerated {
			let frame = Frame {
				header: FrameHeader {
					payload_length: if is_final {
						content.len() as u64
					} else {
						Self::MAX_CHUNK_SIZE_BYTES as u64
					},
					is_final,
				},
				content,
			};
			self.write_frame(frame).await?;
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::{MessageReader, MessageWriter};
	use crate::eventful::{Server, TransportConnection};
	use wtransport::{
		stream::BiStream, ClientConfig, Endpoint, Identity, RecvStream,
		ServerConfig,
	};

	#[test]
	fn test_read_and_write() {
		tokio::runtime::Builder::new_multi_thread()
			.enable_all()
			.build()
			.unwrap()
			.block_on(async {
				let config = ServerConfig::builder()
					.with_bind_default(4433)
					.with_identity(
						&Identity::load_pemfiles(
							"certs/cert.pem",
							"certs/key.pem",
						)
						.await
						.unwrap(),
					)
					.build();
				let fx_msg = String::from("Hi, this is sample text");

				let mut server = Server::new(config).unwrap();
				server.on_connection(|mut connection| {
					connection.on_bi_connection(handle_bi_cnxn).await.unwrap();
				});

				tokio::spawn(async {
					let config = ServerConfig::builder()
						.with_bind_default(4433)
						.with_identity(
							&Identity::load_pemfiles(
								"certs/cert.pem",
								"certs/key.pem",
							)
							.await
							.unwrap(),
						)
						.build();

					let server = Endpoint::server(config).unwrap();

					let connection = server
						.accept()
						.await
						.await
						.unwrap()
						.accept()
						.await
						.unwrap();
					let (mut send, _) = connection.accept_bi().await.unwrap();
					send.write_message(fx_msg).await.unwrap();
				});

				tokio::spawn(async {
					let client =
						Endpoint::client(ClientConfig::default()).unwrap();
					let connection =
						client.connect("https://localhost:4433").await.unwrap();
					loop {
						let (_, mut recv) =
							connection.open_bi().await.unwrap().await.unwrap();
						let mut s = String::new();
						s += std::str::from_utf8(
							&recv.read_message().await.unwrap(),
						)
						.unwrap();
					}
				});
			});
	}

	async fn handle_bi_cnxn(bi_stream: BiStream) {
		let (mut send, _) = bi_stream.split();
		let fx_msg = String::from("Hi, this is sample text");
		send.write_message(fx_msg).await.unwrap();
	}
}

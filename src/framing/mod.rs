use std::{error::Error, future::Future};

use wtransport::{RecvStream, SendStream};

// TODO: make two submodules, use two approaches:
// - implement trait to take asynchronous callback and stream, and call the calback for each frame
// received after parsing
// - check implementation of tungstenite-rs and try to implement same

pub struct FrameHeader {
	is_final: bool,
	payload_length_bytes: u8,
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
}

pub trait FrameReader {
	fn read_frame(
		&mut self,
	) -> impl Future<
		Output = Result<Frame, wtransport::error::StreamReadExactError>,
	> + Send;
}

pub trait FrameWriter {
	fn write_frame(
		&mut self,
		buffer: impl AsRef<[u8]> + Send,
	) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
}

pub trait MessageReader {
	const MAX_MESSAGE_SIZE_BYTES: usize = 65535;
	fn read_message(
		&mut self,
	) -> impl Future<Output = Result<Vec<u8>, MessageReadError>> + Send;
}

pub trait MessageWriter {
	const MAX_CHUNK_SIZE_BYTES: usize = 1024;
	fn write_message(
		&mut self,
		buffer: impl AsRef<[u8]> + Send,
	) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
}

impl FrameReader for RecvStream {
	#[inline]
	async fn read_frame(
		&mut self,
	) -> Result<Frame, wtransport::error::StreamReadExactError> {
		let mut header_bytes = [0];
		self.read_exact(&mut header_bytes).await?;

		// didn't need any bitwise operations :D
		let header = FrameHeader {
			is_final: header_bytes[0] > 128,
			payload_length_bytes: header_bytes[0] - 128,
		};

		let extended_payload_length_bytes: u8 = match header.payload_length_bytes {
			126 => 2,
			127 => 8,
			_ => 0,
		};

		let payload_length = if extended_payload_length_bytes == 0 {
			header.payload_length_bytes
		} else {
			extended_payload_length_bytes
		};

		let mut content_length = vec![0; payload_length as usize];

		self.read_exact(&mut content_length).await?;

        // THIS IS WRONG! DEFINITELY!
		let mut content =
			vec![0; usize::from_be_bytes(content_length.try_into().unwrap())]
				.into_boxed_slice();

		self.read_exact(&mut content);

		Ok(Frame { content, header })
	}
}

impl FrameWriter for SendStream {
	async fn write_frame(
		&mut self,
		buffer: impl AsRef<[u8]>,
	) -> Result<(), Box<dyn Error>> {
		let buffer = buffer.as_ref();
		let payload_length = buffer.len();

		// THIS IS ALL WRONG! idk it might be wrong but lgtm
		let (extra_bytes, payload_length_length): (u8, u8) =
			match payload_length {
				0..=125 => (0, payload_length as u8),
				126..=65535 => (2, 126),
				_ => (8, 127),
			};

		self.write_all(&[128u8 /* or 0b1000_0000 */ | payload_length_length])
			.await?;
		if extra_bytes == 2 {
			self.write_all(&(payload_length as u16).to_be_bytes())
				.await?;
		}
		if extra_bytes == 8 {
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
	async fn read_message(&mut self) -> Result<Vec<u8>, MessageReadError> {
		let mut message = Vec::new();
		let mut current_len = 0;
		loop {
			let frame = self.read_frame().await?;
			let is_final = frame.header().is_final();
			let content = frame.get_content();
			current_len += frame.header().payload_length_bytes;
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
	async fn write_message(
		&mut self,
		buffer: impl AsRef<[u8]>,
	) -> Result<(), Box<dyn Error>> {
		todo!()
	}
}

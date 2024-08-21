use std::{error::Error, future::Future, ops::Deref};

use wtransport::{stream::BiStream, RecvStream, SendStream};

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

pub trait FrameReader {
	fn read_frame(
		&mut self,
	) -> impl Future<Output = Result<Frame, Box<dyn Error>>> + Send;
}

pub trait FrameWriter {
	fn write_frame(
		&mut self,
		buffer: impl AsRef<[u8]> + Send,
	) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
}

pub trait MessageReader<const MAX_MESSAGE_SIZE_BYTES: usize = 65535> {
	fn read_message(
		&mut self,
	) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
}

pub trait MessageWriter<const MAX_MESSAGE_SIZE_BYTES: usize = 65535> {
	fn write_message(
		&mut self,
	) -> impl Future<Output = Result<(), Box<dyn Error>>> + Send;
}

impl FrameReader for RecvStream {
	#[inline]
	async fn read_frame(&mut self) -> Result<Frame, Box<dyn Error>> {
		let mut header_bytes = [0];
		self.read_exact(&mut header_bytes).await?;

		// didn't need any bitwise operations :D
		let header = FrameHeader {
			is_final: header_bytes[0] > 128,
			payload_length_bytes: header_bytes[0] - 128,
		};

		let len: usize = match header.payload_length_bytes {
			126 => 8 * 2,
			127 => 8 * 8,
			len => 8 * len as usize,
		};

		let mut content = vec![0; len].into_boxed_slice();

		self.read_exact(&mut content).await?;

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
				..126 => (0, payload_length as u8),
				126..65536 => (16, 126),
				65536.. => (64, 127),
			};

		let mut payload = Vec::with_capacity(
			1 /* header */ + extra_bytes as usize /* extended payload length */ + payload_length,
		);

		payload.push(128u8 | payload_length_length);
		payload.extend_from_slice(buffer);

		self.write_all(buffer).await?;
		Ok(())
	}
}

impl MessageReader for RecvStream {
	async fn read_message(&mut self) -> Result<(), Box<dyn Error>> {
		loop {
			let frame = self.read_frame().await?;
			if frame.header.is_final {
				break;
			}
		}

		todo!()
	}
}

use std::io;
use wtransport::{Connection, Endpoint, ServerConfig};

pub struct Server {
	endpoint: Endpoint<wtransport::endpoint::endpoint_side::Server>,
}

impl Server {
	pub fn new(config: ServerConfig) -> io::Result<Self> {
		Ok(Self {
			endpoint: Endpoint::server(config)?,
		})
	}

	pub async fn accept(
		&self,
	) -> Result<Connection, wtransport::error::ConnectionError> {
		self.endpoint.accept().await.await?.accept().await
	}

	pub async fn on_connection<F>(&self, mut callback: F) -> io::Result<()>
	where
		F: FnMut(Connection) -> io::Result<()>,
	{
		loop {
			match self.accept().await {
				Ok(connection) => callback(connection)?,
				Err(e) => eprintln!("Error accepting connection: {}", e),
			}
		}
	}
}

pub struct TransportConnection {
	connection: Connection,
}

impl TransportConnection {
	pub fn new(connection: Connection) -> Self {
		Self { connection }
	}

	pub async fn on_bi_stream<F>(&self, mut callback: F) -> io::Result<()>
	where
		F: FnMut(wtransport::SendStream) -> io::Result<()>,
	{
		while let Ok((stream, _)) = self.connection.accept_bi().await {
			callback(stream)?;
		}
		Ok(())
	}

	pub async fn on_uni_stream<F>(&self, mut callback: F) -> io::Result<()>
	where
		F: FnMut(wtransport::RecvStream) -> io::Result<()>,
	{
		while let Ok(stream) = self.connection.accept_uni().await {
			callback(stream)?;
		}
		Ok(())
	}
}

// Remove the `add` function as it seems unrelated to the WebTransport wrapper

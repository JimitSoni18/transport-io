pub mod claude;
pub mod eventful;

pub use wtransport::ServerConfig;
use wtransport::{Connection, Endpoint};

pub struct Server {
	endpoint_server: Endpoint<wtransport::endpoint::endpoint_side::Server>,
}

pub struct TransportConnection {
	connection: wtransport::connection::Connection,
}

impl TransportConnection {
	#[inline]
	pub async fn accept_bi(
		&self,
	) -> Result<
		(wtransport::SendStream, wtransport::RecvStream),
		wtransport::error::ConnectionError,
	> {
		self.connection.accept_bi().await
	}

	#[inline]
	pub async fn accept_uni(
		&self,
	) -> Result<wtransport::RecvStream, wtransport::error::ConnectionError> {
		self.connection.accept_uni().await
	}
}

impl Server {
	pub fn new(server_config: ServerConfig) -> std::io::Result<Self> {
		let endpoint_server = Endpoint::server(server_config)?;
		Ok(Self { endpoint_server })
	}

	pub async fn on_connection(
		self,
		_f: impl Fn(Connection),
	) -> Result<TransportConnection, wtransport::error::ConnectionError> {
		let connection =
			self.endpoint_server.accept().await.await?.accept().await?;

		// f(connection);
		// connection.open_bi();
		// connection.accept_bi();
		Ok(TransportConnection { connection })
	}
}

#[cfg(test)]
mod tests {}

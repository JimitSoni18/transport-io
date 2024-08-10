// This is trash
use std::future::Future;
use wtransport::{
	error::ConnectionError, stream::BiStream, Endpoint, RecvStream,
	ServerConfig, VarInt,
};

pub struct Server<T> {
	endpoint_server: Endpoint<wtransport::endpoint::endpoint_side::Server>,
	handle_connection: Option<T>,
}

impl<T> Server<T> {
	pub fn new(server_config: ServerConfig) -> std::io::Result<Self> {
		Ok(Self {
			endpoint_server: Endpoint::server(server_config)?,
			handle_connection: None,
		})
	}

	pub async fn on_connection<U, B>(self: &mut Self, f: T)
	where
		T: Fn(TransportConnection<U, B>),
	{
		self.handle_connection = Some(f);
	}

	pub async fn serve<U, B, D>(self) -> Result<(), ConnectionError>
	where
		T: Fn(TransportConnection<U, B>),
	{
		loop {
			let connection =
				self.endpoint_server.accept().await.await?.accept().await?;
			let transport_connection = TransportConnection {
				connection,
				on_bi_connection: None,
				on_uni_connection: None,
			};

			if let Some(handle_connection) = &self.handle_connection {
				handle_connection(transport_connection)
			}
		}
	}
}

impl<U, B> Drop for TransportConnection<U, B> {
	fn drop(&mut self) {
		self.connection.close(VarInt::from_u32(69), b"because");
	}
}

pub struct TransportConnection<U, B> {
	connection: wtransport::connection::Connection,
	on_bi_connection: Option<B>,
	on_uni_connection: Option<U>,
}

impl<U, B> TransportConnection<U, B> {
	pub async fn on_bi_connection<F>(
		&mut self,
		f: B,
	) -> Result<(), ConnectionError>
	where
		B: Fn(BiStream) -> F + Send + 'static,
		F: Future<Output = ()> + Send + 'static,
	{
		if self.on_bi_connection.is_some() {
            // FIXME: should not panic!
			panic!("already had a bi_connection handler, cannot override")
		}
		self.on_bi_connection = Some(f);
		if let Some(handle_bi_connection) = &self.on_bi_connection {
			tokio::spawn(handle_bi_connection(
				self.connection.accept_bi().await?.into(),
			));
		}

		Ok(())
	}

	pub async fn on_uni_connection<F>(
		&mut self,
		f: U,
	) -> Result<(), ConnectionError>
	where
		U: Fn(RecvStream) -> F,
		F: Future<Output = ()> + Send + 'static,
	{
		if self.on_uni_connection.is_some() {
            // FIXME: should not panic!
			panic!("already had a uni_connection handler, cannot override")
		}
		self.on_uni_connection = Some(f);
		if let Some(handle_uni_connection) = &self.on_uni_connection {
			tokio::spawn(handle_uni_connection(
				self.connection.accept_uni().await?,
			));
		}

		Ok(())
	}

	pub fn close(self, error_code: VarInt, reason: &[u8]) {
		self.connection.close(error_code, reason);
	}
}

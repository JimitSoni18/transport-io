// This is less trash
use std::future::Future;
use wtransport::{
	error::ConnectionError, stream::BiStream, Endpoint, RecvStream,
	ServerConfig,
};

pub struct Server<T> {
	endpoint_server: Endpoint<wtransport::endpoint::endpoint_side::Server>,
	handle_connection: Option<T>,
}

impl<T> Server<T> {
	pub fn new(config: ServerConfig) -> std::io::Result<Self> {
		Ok(Server {
			handle_connection: None,
			endpoint_server: Endpoint::server(config)?,
		})
	}

	pub fn on_connection(&mut self, f: T)
	where
		T: Fn(TransportConnection),
	{
		self.handle_connection = Some(f);
	}

	pub async fn serve(self) -> Result<(), ConnectionError>
	where
		T: Fn(TransportConnection),
	{
		if let Some(handle_connection) = &self.handle_connection {
			loop {
				let connection =
					self.endpoint_server.accept().await.await?.accept().await?;
				let transport_connection = TransportConnection {
					connection,
					bi_connection_handler_spawned: false,
					uni_connection_handler_spawned: false,
				};

				handle_connection(transport_connection);
			}
		}
		Ok(())
	}
}

pub struct TransportConnection {
	connection: wtransport::connection::Connection,
	bi_connection_handler_spawned: bool,
	uni_connection_handler_spawned: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum UniStreamHandlerError {
	#[error(transparent)]
	TransportError(ConnectionError),
	#[error("handler is already set")]
	HandlerAlreadySpawned,
}

impl From<ConnectionError> for UniStreamHandlerError {
	fn from(value: ConnectionError) -> Self {
		Self::TransportError(value)
	}
}

impl TransportConnection {
	pub async fn on_uni_connection<U, F>(
		&mut self,
		f: U,
	) -> Result<(), UniStreamHandlerError>
	where
		U: Fn(RecvStream) -> F,
		F: Future<Output = ()> + Send + 'static,
	{
		if self.uni_connection_handler_spawned {
			return Err(UniStreamHandlerError::HandlerAlreadySpawned);
		}
		self.uni_connection_handler_spawned = true;
		loop {
			let result_recv_stream = self.connection.accept_uni().await;
			if let Ok(recv_stream) = result_recv_stream {
				tokio::spawn(f(recv_stream));
			}
		}
	}

	pub async fn on_bi_connection<B, F>(
		&mut self,
		f: B,
	) -> Result<(), UniStreamHandlerError>
	where
		B: Fn(BiStream) -> F,
		F: Future<Output = ()> + Send + 'static,
	{
		if self.bi_connection_handler_spawned {
			return Err(UniStreamHandlerError::HandlerAlreadySpawned);
		}
		self.bi_connection_handler_spawned = true;
		loop {
			let result_recv_stream = self.connection.accept_bi().await;
			if let Ok(recv_stream) = result_recv_stream {
				tokio::spawn(f(recv_stream.into()));
			}
		}
	}
}

// This is trash
// use std::future::Future;
// use wtransport::{
// 	error::ConnectionError, stream::BiStream, Endpoint, RecvStream,
// 	ServerConfig, VarInt,
// };
// 
// pub struct Server<T> {
// 	endpoint_server: Endpoint<wtransport::endpoint::endpoint_side::Server>,
// 	handle_connection: Option<T>,
// }
// 
// impl<T> Server<T> {
// 	pub fn new(server_config: ServerConfig) -> std::io::Result<Self> {
// 		Ok(Self {
// 			endpoint_server: Endpoint::server(server_config)?,
// 			handle_connection: None,
// 		})
// 	}
// 
// 	pub async fn on_connection<U, B, F>(&mut self, f: T)
// 	where
// 		T: Fn(TransportConnection<U, B>),
// 		B: Fn(BiStream) -> F,
// 		F: Future<Output = ()>,
// 	{
// 		self.handle_connection = Some(f);
// 	}
// 
// 	pub async fn serve<U, B, D>(self) -> Result<(), ConnectionError>
// 	where
// 		T: Fn(TransportConnection<U, B>),
// 	{
// 		if let Some(handle_connection) = &self.handle_connection {
// 			loop {
// 				let connection =
// 					self.endpoint_server.accept().await.await?.accept().await?;
// 				let transport_connection = TransportConnection {
// 					connection,
// 					bi_connection_handler: None,
// 					uni_connection_handler: None,
// 				};
// 
// 				handle_connection(transport_connection)
// 			}
// 		}
// 		Ok(())
// 	}
// }
// 
// impl<U, B> Drop for TransportConnection<U, B> {
// 	fn drop(&mut self) {
// 		self.connection.close(VarInt::from_u32(69), b"because");
// 	}
// }
// 
// pub struct TransportConnection<
// 	U = fn(RecvStream) -> Box<dyn Future<Output = ()>>,
// 	B = fn(BiStream) -> Box<dyn Future<Output =()>>,
// > {
// 	connection: wtransport::connection::Connection,
// 	bi_connection_handler: Option<B>,
// 	uni_connection_handler: Option<U>,
// }
// 
// impl<U, B> TransportConnection<U, B> {
// 	pub async fn on_bi_connection<F>(
// 		&mut self,
// 		f: B,
// 	) -> Result<(), ConnectionError>
// 	where
// 		B: Fn(BiStream) -> F + Send + 'static,
// 		F: Future<Output = ()> + Send + 'static,
// 	{
// 		if self.bi_connection_handler.is_some() {
// 			// FIXME: should not panic!
// 			panic!("already had a bi_connection handler, cannot override")
// 		}
// 		self.bi_connection_handler = Some(f);
// 		if let Some(handle_bi_connection) = &self.bi_connection_handler {
// 			tokio::spawn(handle_bi_connection(
// 				self.connection.accept_bi().await?.into(),
// 			));
// 		}
// 
// 		Ok(())
// 	}
// 
// 	pub async fn on_uni_connection<F>(
// 		&mut self,
// 		f: U,
// 	) -> Result<(), ConnectionError>
// 	where
// 		U: Fn(RecvStream) -> F,
// 		F: Future<Output = ()> + Send + 'static,
// 	{
// 		if self.uni_connection_handler.is_some() {
// 			// FIXME: should not panic!
// 			panic!("already had a uni_connection handler, cannot override")
// 		}
// 		self.uni_connection_handler = Some(f);
// 		if let Some(handle_uni_connection) = &self.uni_connection_handler {
// 			tokio::spawn(handle_uni_connection(
// 				self.connection.accept_uni().await?,
// 			));
// 		}
// 
// 		Ok(())
// 	}
// 
// 	pub fn close(self, error_code: VarInt, reason: &[u8]) {
// 		self.connection.close(error_code, reason);
// 	}
// }

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
pub enum ConnectionHandlerError {
	#[error(transparent)]
	TransportError(ConnectionError),
	#[error("handler is already set")]
	HandlerAlreadySpawned,
}

impl From<ConnectionError> for ConnectionHandlerError {
	fn from(value: ConnectionError) -> Self {
		Self::TransportError(value)
	}
}

impl TransportConnection {
	pub async fn on_uni_connection<U, F>(
		&mut self,
		f: U,
	) -> Result<(), ConnectionHandlerError>
	where
		U: Fn(RecvStream) -> F,
		F: Future<Output = ()> + Send + 'static,
	{
		if self.uni_connection_handler_spawned {
			return Err(ConnectionHandlerError::HandlerAlreadySpawned);
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
	) -> Result<(), ConnectionHandlerError>
	where
		B: Fn(BiStream) -> F,
		F: Future<Output = ()> + Send + 'static,
	{
		if self.bi_connection_handler_spawned {
			return Err(ConnectionHandlerError::HandlerAlreadySpawned);
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

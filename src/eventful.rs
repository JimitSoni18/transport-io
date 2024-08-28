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
	connection: Option<TransportConnection>,
}

#[derive(thiserror::Error, Debug)]
pub enum ServerError {
	#[error("connection handler not provided")]
	ServerHandlerNotProvided,
	#[error("no stream handler provided")]
	ConnectionHandlerNotProvided,
}

impl<T> Server<T> {
	pub fn new(config: ServerConfig) -> std::io::Result<Self> {
		Ok(Server {
			handle_connection: None,
			endpoint_server: Endpoint::server(config)?,
			connection: None,
		})
	}

	pub fn on_connection(&mut self, f: T)
	where
		T: Fn(TransportConnection),
	{
		self.handle_connection = Some(f);
	}

	pub async fn serve(self) -> Result<(), ServerError>
	where
		T: Fn(TransportConnection),
	{
		let connection = self.connection.unwrap();

		let TransportConnection {
			bi_connection_handler,
			uni_connection_handler,
		} = connection;

        if bi_connection_handler.is_none() && uni_connection_handler.is_none() {
            return Err(ServerError::ConnectionHandlerNotProvided);
        }

		if let Some(handle_connection) = &self.handle_connection {
			loop {
				let transport_connection = TransportConnection {
					bi_connection_handler: None,
					uni_connection_handler: None,
				};

				handle_connection(transport_connection);
			}
		}
		Err(ServerError::ServerHandlerNotProvided)
	}
}

type DefaultUniConnectionHandler =
	fn(RecvStream) -> Box<dyn Future<Output = ()>>;
type DefaultBiConnectionHandler = fn(BiStream) -> Box<dyn Future<Output = ()>>;

pub struct TransportConnection<
	U = DefaultUniConnectionHandler,
	B = DefaultBiConnectionHandler,
> {
	bi_connection_handler: Option<B>,
	uni_connection_handler: Option<U>,
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

impl<U, B> TransportConnection<U, B> {
	pub async fn on_uni_connection<F>(
		&mut self,
		f: U,
	) -> Result<(), ConnectionHandlerError>
	where
		U: Fn(RecvStream) -> F,
		F: Future<Output = ()> + Send + 'static,
	{
		if self.uni_connection_handler.is_some() {
			return Err(ConnectionHandlerError::HandlerAlreadySpawned);
		}
		self.uni_connection_handler = Some(f);
		Ok(())
	}

	pub async fn on_bi_connection<F>(
		&mut self,
		f: B,
	) -> Result<(), ConnectionHandlerError>
	where
		B: Fn(BiStream) -> F,
		F: Future<Output = ()> + Send + 'static,
	{
		if self.bi_connection_handler.is_some() {
			return Err(ConnectionHandlerError::HandlerAlreadySpawned);
		}
		self.bi_connection_handler = Some(f);
		Ok(())
	}
}

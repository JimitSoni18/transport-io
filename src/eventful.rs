// This is less trash
use std::future::Future;
use wtransport::{
	error::ConnectionError, stream::BiStream, Connection, Endpoint, RecvStream, ServerConfig
};

mod handler;
mod state;

use handler::Handler;
use state::State;

pub struct Server<T> {
	endpoint_server: Endpoint<wtransport::endpoint::endpoint_side::Server>,
	connection_handler: Option<T>,
	connection: Option<TransportConnection>,
}

type DefaultServerConnectionHandler =
	fn(TransportConnection) -> Box<dyn Future<Output = ()>>;
type DefaultServerErrorConnectionHandler =
	fn(ConnectionError) -> Box<dyn Future<Output = ()>>;

pub struct ServerBuilder<
	T = DefaultServerConnectionHandler,
	E = DefaultServerErrorConnectionHandler,
	S = (),
> {
	connection_handler: Option<T>,
	error_handler: Option<E>,
	config: ServerConfig,
	state: State<S>,
}

#[derive(thiserror::Error, Debug)]
pub enum ServerError {
	#[error("connection handler not provided")]
	ServerHandlerNotProvided,
	#[error("no stream handler provided")]
	ConnectionHandlerNotProvided,
}

impl<T> Server<T> {
	pub fn builder(config: ServerConfig) -> ServerBuilder {
		ServerBuilder {
			connection_handler: None,
			error_handler: None,
			config,
			state: State(()),
		}
	}

	pub async fn serve(self) -> Result<(), ServerError>
	where
		T: Fn(TransportConnection),
	{
		todo!();
	}
}

impl<T, E, S> ServerBuilder<T, E, S> {
	pub fn on_connection(&mut self, f: T)
	where
		T: Handler,
	{
		self.connection_handler = Some(f);
	}

	pub fn on_error(&mut self, f: E)
	where
		E: Handler,
	{
		self.error_handler = Some(f);
	}

	pub fn with_global_state(&mut self, state: S) {
		self.state = State(state);
	}
}

type DefaultUniConnectionHandler =
	fn(RecvStream) -> Box<dyn Future<Output = ()>>;
type DefaultBiConnectionHandler = fn(BiStream) -> Box<dyn Future<Output = ()>>;

pub struct TransportConnection {
	uni_spawned: bool,
	bi_spawned: bool,
    connection: Connection,
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
	pub async fn on_uni_connection<F, Fut>(
		&mut self,
		f: F,
	) -> Result<(), ConnectionHandlerError>
	where
		F: Fn(RecvStream) -> Fut + Send + Clone + 'static,
		Fut: Future<Output = ()> + Send + 'static,
	{
		if self.uni_spawned {
			return Err(ConnectionHandlerError::HandlerAlreadySpawned);
		}
		self.uni_spawned = true;
        tokio::spawn(async {
            loop {
            }
        });
        todo!();
	}

	pub async fn on_bi_connection<F, Fut>(
		&mut self,
		f: F,
	) -> Result<(), ConnectionHandlerError>
	where
		F: Fn(BiStream) -> Fut + Send + Clone + 'static,
		Fut: Future<Output = ()> + Send + 'static,
	{
        if self.bi_spawned {
            return Err(ConnectionHandlerError::HandlerAlreadySpawned);
        }
        self.bi_spawned = true;
		todo!()
	}
}

// TODO: should I implement Future for TransportConnection, where it starts spawning tasks?
impl Future for TransportConnection {
    type Output = ();
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        todo!()
    }
}

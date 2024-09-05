use std::future::Future;

use super::{state::State, TransportConnection};

pub trait Handler<S = ()> {
	fn call(
		&self,
		connection: TransportConnection,
		state: State<S>,
	) -> impl Future<Output = ()>;
}

impl<F, S> Handler<S> for fn(TransportConnection) -> F
where
	F: Future<Output = ()>,
{
	fn call(
		&self,
		connection: TransportConnection,
		_: State<S>,
	) -> impl Future<Output = ()> {
		self(connection)
	}
}

impl<F, S> Handler<S> for fn(TransportConnection, State<S>) -> F
where
	F: Future<Output = ()>,
{
	fn call(
		&self,
		connection: TransportConnection,
		state: State<S>,
	) -> impl Future<Output = ()> {
		self(connection, state)
	}
}

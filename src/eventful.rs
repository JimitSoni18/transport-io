use wtransport::{error::ConnectionError, stream::BiStream, Endpoint, ServerConfig};

pub struct Server<T> {
    endpoint_server: Endpoint<wtransport::endpoint::endpoint_side::Server>,
    on_connection: T,
}

impl<T> Server<T> {
	pub fn new(server_config: ServerConfig) -> std::io::Result<Self> {
		let endpoint_server = Endpoint::server(server_config)?;
        todo!()
		// Ok(Self { endpoint_server })
	}

	pub async fn on_connection<U: Fn(), B: Fn(BiStream), D: Fn()>(
		&self,
		f: impl Fn(&TransportConnection<U, B, D>),
	) -> Result<(), ConnectionError> {
		let connection = self.endpoint_server.accept().await.await?.accept().await?;
        let transport_connection = TransportConnection {
			connection,
			on_bi_connection: None,
			on_uni_connection: None,
			on_dgram: None,
		};
		f(&transport_connection);

        Ok(())
	}

    pub async fn serve(self) -> Result<(), ConnectionError> {
        let connection = self.endpoint_server.accept().await.await?.accept().await?;
        let transport_connection = TransportConnection {
            connection,
            on_bi_connection: todo!(),
            on_uni_connection: todo!(),
            on_dgram: todo!(),
        };
        Ok(())
    }
}

pub struct TransportConnection<U, B, D> {
	connection: wtransport::connection::Connection,
	on_bi_connection: Option<B>,
	on_uni_connection: Option<U>,
	on_dgram: Option<D>,
}

impl<U: Fn(), B: Fn(BiStream), D: Fn()> TransportConnection<U, B, D> {
	pub async fn on_bi_connection(&mut self, f: B) {
		self.on_bi_connection = Some(f);
	}

	pub fn on_uni_connection(&mut self, f: U) {
		self.on_uni_connection = Some(f);
	}

	pub fn on_dgram(&mut self, f: D) {
		self.on_dgram = Some(f);
	}
}

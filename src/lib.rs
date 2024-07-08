pub mod claude;

pub use wtransport::ServerConfig;
use wtransport::{Connection, Endpoint};

pub struct Server {
    endpoint_server: Endpoint<wtransport::endpoint::endpoint_side::Server>,
}

pub struct TransportConnection<B: Fn(), U: Fn()> {
    on_bi_connection: Option<B>,
    on_uni_connection: Option<U>,
}

impl<B: Fn(), U: Fn()> TransportConnection<B, U> {
    pub fn on_bi_connection(&mut self, f: B) {
        self.on_bi_connection = Some(f)
    }

    pub fn on_uni_connection(&mut self, f: U) {
        self.on_uni_connection = Some(f);
    }
}

impl Server {
    pub fn new(server_config: ServerConfig) -> std::io::Result<Self> {
        let endpoint_server = Endpoint::server(server_config)?;
        Ok(Self { endpoint_server })
    }

    pub async fn on_connection<B: Fn(), U: Fn()>(
        self,
        f: impl Fn(Connection),
    ) -> Result<TransportConnection<B, U>, wtransport::error::ConnectionError> {
        let connection = self.endpoint_server.accept().await.await?.accept().await?;

        f(connection);
        Ok(TransportConnection {
            on_uni_connection: None,
            on_bi_connection: None,
        })
    }
}

#[cfg(test)]
mod tests {
}

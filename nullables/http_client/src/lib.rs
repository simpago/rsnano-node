use anyhow::anyhow;
use reqwest::{IntoUrl, Method, StatusCode};
use rsnano_output_tracker::{OutputListenerMt, OutputTrackerMt};
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, sync::Arc};

pub use reqwest::Url;

pub struct HttpClient {
    strategy: HttpClientStrategy,
    request_listener: OutputListenerMt<TrackedRequest>,
}

impl HttpClient {
    pub fn new() -> Self {
        Self::new_with_strategy(HttpClientStrategy::Real(reqwest::Client::new()))
    }

    pub fn new_null() -> Self {
        Self::new_with_strategy(HttpClientStrategy::Nulled(HttpClientStub::with_response(
            ConfiguredResponse::default(),
        )))
    }

    fn new_with_strategy(strategy: HttpClientStrategy) -> Self {
        Self {
            strategy,
            request_listener: OutputListenerMt::new(),
        }
    }

    pub fn null_builder() -> NulledHttpClientBuilder {
        NulledHttpClientBuilder {
            responses: HashMap::new(),
        }
    }

    pub async fn post_json<U: IntoUrl, T: Serialize + ?Sized>(
        &self,
        url: U,
        json: &T,
    ) -> anyhow::Result<Response> {
        let url = url.into_url()?;
        if self.request_listener.is_tracked() {
            self.request_listener.emit(TrackedRequest {
                url: url.clone(),
                method: Method::POST,
                json: serde_json::to_value(json)?,
            });
        }

        match &self.strategy {
            HttpClientStrategy::Real(client) => {
                Ok(client.post(url).json(json).send().await?.into())
            }
            HttpClientStrategy::Nulled(client) => client.get_response(Method::POST, url),
        }
    }

    pub fn track_requests(&self) -> Arc<OutputTrackerMt<TrackedRequest>> {
        self.request_listener.track()
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

enum HttpClientStrategy {
    Real(reqwest::Client),
    Nulled(HttpClientStub),
}

pub struct NulledHttpClientBuilder {
    responses: HashMap<(Url, Method), ConfiguredResponse>,
}

impl NulledHttpClientBuilder {
    pub fn respond(self, response: ConfiguredResponse) -> HttpClient {
        HttpClient::new_with_strategy(HttpClientStrategy::Nulled(HttpClientStub {
            the_only_response: Some(response),
            responses: HashMap::new(),
        }))
    }

    pub fn respond_url(
        mut self,
        method: Method,
        url: impl IntoUrl,
        response: ConfiguredResponse,
    ) -> Self {
        self.responses
            .insert((url.into_url().unwrap(), method), response);
        self
    }

    pub fn finish(self) -> HttpClient {
        HttpClient::new_with_strategy(HttpClientStrategy::Nulled(HttpClientStub {
            the_only_response: None,
            responses: self.responses,
        }))
    }
}

struct HttpClientStub {
    the_only_response: Option<ConfiguredResponse>,
    responses: HashMap<(Url, Method), ConfiguredResponse>,
}

impl HttpClientStub {
    fn with_response(response: ConfiguredResponse) -> Self {
        Self {
            the_only_response: Some(response),
            responses: HashMap::new(),
        }
    }

    fn get_response(&self, method: Method, url: Url) -> anyhow::Result<Response> {
        let response = if let Some(r) = &self.the_only_response {
            Some(r)
        } else {
            self.responses.get(&(url.clone(), method.clone()))
        };

        response
            .map(|r| r.clone().into())
            .ok_or_else(|| anyhow!("no response configured for {} {}", method, url))
    }
}

#[derive(Clone)]
pub struct TrackedRequest {
    pub url: Url,
    pub method: Method,
    pub json: serde_json::Value,
}

pub struct Response {
    strategy: ResponseStrategy,
}

impl Response {
    pub fn status(&self) -> StatusCode {
        match &self.strategy {
            ResponseStrategy::Real(resp) => resp.status(),
            ResponseStrategy::Nulled(resp) => resp.status,
        }
    }

    pub async fn json<T: DeserializeOwned>(self) -> anyhow::Result<T> {
        match self.strategy {
            ResponseStrategy::Real(resp) => Ok(resp.json().await?),
            ResponseStrategy::Nulled(resp) => resp.json(),
        }
    }
}

enum ResponseStrategy {
    Real(reqwest::Response),
    Nulled(ConfiguredResponse),
}

impl From<reqwest::Response> for Response {
    fn from(value: reqwest::Response) -> Self {
        Self {
            strategy: ResponseStrategy::Real(value),
        }
    }
}

#[derive(Clone)]
pub struct ConfiguredResponse {
    status: StatusCode,
    body: serde_json::Value,
}

impl ConfiguredResponse {
    pub fn new(status: StatusCode, json: impl Serialize) -> Self {
        Self {
            status,
            body: serde_json::to_value(json).unwrap(),
        }
    }
    pub fn json<T: DeserializeOwned>(&self) -> anyhow::Result<T> {
        let deserialized = serde_json::from_value(self.body.clone())?;
        Ok(deserialized)
    }
}

impl Default for ConfiguredResponse {
    fn default() -> Self {
        Self {
            status: StatusCode::OK,
            body: serde_json::Value::Null,
        }
    }
}

impl From<ConfiguredResponse> for Response {
    fn from(value: ConfiguredResponse) -> Self {
        Self {
            strategy: ResponseStrategy::Nulled(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn make_real_request() {
        let port = get_available_port().await;
        let _server = test_http_server::start(("0.0.0.0", port)).await;

        let client = HttpClient::new();
        let result = client
            .post_json(format!("http://127.0.0.1:{}", port), &vec!["hello"])
            .await
            .unwrap();
        assert_eq!(result.status(), StatusCode::OK);
        let response = result.json::<Vec<String>>().await.unwrap();
        assert_eq!(response, vec!["hello".to_string(), "world".to_string()]);
    }

    #[tokio::test]
    async fn track_requests() {
        let client = HttpClient::new_null();
        let tracker = client.track_requests();
        let target_url: Url = "http://127.0.0.1:42/foobar".parse().unwrap();
        let data = vec![1, 2, 3];

        client.post_json(target_url.clone(), &data).await.unwrap();

        let requests = tracker.output();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url, target_url);
        assert_eq!(requests[0].method, Method::POST);
        assert_eq!(requests[0].json, serde_json::to_value(&data).unwrap());
    }

    mod nullability {
        use super::*;

        #[tokio::test]
        async fn can_be_nulled() {
            let client = HttpClient::new_null();
            let response = client
                .post_json("http://127.0.0.1:42", "foobar")
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        #[tokio::test]
        async fn return_configured_json_response() {
            let client = HttpClient::null_builder()
                .respond(ConfiguredResponse::new(StatusCode::OK, vec![1, 2, 3]));

            let response = client.post_json("http://127.0.0.1:42", "").await.unwrap();
            assert_eq!(response.json::<Vec<i32>>().await.unwrap(), vec![1, 2, 3]);
        }
    }

    mod test_http_server {
        use axum::{routing::post, Json, Router};
        use tokio::{
            net::{TcpListener, ToSocketAddrs},
            sync::oneshot,
        };
        use tokio_util::sync::CancellationToken;

        pub(crate) struct DropGuard {
            cancel_token: CancellationToken,
        }

        impl Drop for DropGuard {
            fn drop(&mut self) {
                self.cancel_token.cancel();
            }
        }

        pub(crate) async fn start(addr: impl ToSocketAddrs + Send + 'static) -> DropGuard {
            let guard = DropGuard {
                cancel_token: CancellationToken::new(),
            };
            let cancel_token = guard.cancel_token.clone();
            let (tx_ready, rx_ready) = oneshot::channel::<()>();

            tokio::spawn(async move { run_server(addr, cancel_token, tx_ready).await });

            rx_ready.await.unwrap();

            guard
        }

        async fn run_server(
            addr: impl ToSocketAddrs,
            cancel_token: CancellationToken,
            tx_ready: oneshot::Sender<()>,
        ) {
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            tx_ready.send(()).unwrap();
            tokio::select! {
                _ = serve(listener) => { },
                _ = cancel_token.cancelled() => { }
            }
        }

        async fn serve(tcp_listener: TcpListener) {
            let app = Router::new().route("/", post(root));
            axum::serve(tcp_listener, app).await.unwrap()
        }

        async fn root(Json(mut payload): Json<Vec<String>>) -> Json<Vec<String>> {
            payload.push("world".to_string());
            Json(payload)
        }
    }

    async fn get_available_port() -> u16 {
        for port in 1025..65535 {
            if is_port_available(port).await {
                return port;
            }
        }

        panic!("Could not find an available port");
    }

    async fn is_port_available(port: u16) -> bool {
        match TcpListener::bind(("127.0.0.1", port)).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

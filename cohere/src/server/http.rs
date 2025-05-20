use axum::{
    Router,
    extract::Request,
    http::{self, StatusCode},
    middleware::Next,
    response::{Json, Response},
    routing::get,
    serve,
};
use opentelemetry::{global, propagation::Extractor};
use serde_json::{Value, json};
use std::net::SocketAddr;
use tracing::info;
use tracing_opentelemetry::OpenTelemetrySpanExt;

const HEALTHCHECK_PATH: &str = "/healthz";

pub fn new_http() -> Router {
    Router::new()
}

async fn healthcheck() -> Json<Value> {
    Json(json!({
        "success": true,
    }))
}

fn get_client_ip(req: &Request) -> String {
    req.extensions()
        .get::<axum::extract::connect_info::ConnectInfo<SocketAddr>>()
        .unwrap()
        .ip()
        .to_string()
}

fn get_user_agent(req: &Request) -> String {
    req.headers()
        .get(http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_owned()
}

async fn http_request_log(req: Request, next: Next) -> Result<Response, StatusCode> {
    let ip = get_client_ip(&req);
    let user_agent = get_user_agent(&req);

    let method = req.method().to_string();
    let path = req.uri().path().to_owned();

    let start = std::time::Instant::now();
    let response = next.run(req).await;
    let latency_ms = start.elapsed().as_millis();

    let status = response.status().as_u16();

    info!(
        client.address = ip,
        user_agent.original = user_agent,
        "[{}] {} - {} {}ms",
        status,
        method,
        path,
        latency_ms
    );

    Ok(response)
}

async fn http_request_trace(req: Request, next: Next) -> Result<Response, StatusCode> {
    let uri = req.uri();
    if uri.path() == HEALTHCHECK_PATH {
        return Ok(next.run(req).await);
    }

    let ip = get_client_ip(&req);
    let user_agent = get_user_agent(&req);

    let parent_cx = global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(req.headers()))
    });

    let span = tracing::info_span!(
        "http_request",
        otel.name = uri.path(),
        otel.kind = "Server",
        client.address = ip,
        http.request.method = req.method().as_str(),
        // http.route = "/users/{id}",
        network.protocal.version = format!("{:?}", req.version())
            .strip_prefix("HTTP/")
            .unwrap_or("unknown"),
        server.address = uri.host(),
        url.path = uri.path(),
        url.query = uri.query(),
        url.scheme = uri.scheme_str(),
        user_agent.original = user_agent,
    );
    span.set_parent(parent_cx);

    let _enter = span.enter();

    let response = next.run(req).await;

    span.set_attribute(
        "http.response.status_code",
        response.status().as_u16() as i64,
    );

    Ok(response)
}

struct HeaderExtractor<'a>(&'a http::HeaderMap);

impl Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }
    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|v| v.as_str()).collect()
    }
}

pub async fn serve_http(app: Router, port: u16) -> anyhow::Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("server listening on {}", addr);

    serve(
        listener,
        app.route(HEALTHCHECK_PATH, get(healthcheck))
            .layer(axum::middleware::from_fn(http_request_log))
            .layer(axum::middleware::from_fn(http_request_trace))
            .into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(super::shutdown_signal())
    .await?;

    Ok(())
}

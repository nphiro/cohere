use axum::http;
use http::HeaderMap;
use opentelemetry::{global, propagation::Injector, trace::Status};
use reqwest::{Error, RequestBuilder, Response};
use tracing::info;
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub async fn send_http(builder: RequestBuilder) -> Result<Response, Error> {
    let req = match builder.try_clone() {
        Some(clone) => match clone.build() {
            Ok(req) => req,
            Err(e) => return Err(e),
        },
        None => {
            return builder.send().await;
        }
    };

    let method = req.method().as_str();
    let path = req.url().path();
    let query = req.url().query();

    let span = tracing::info_span!(
        "http_client_request",
        otel.name = format!("{} {}", req.method(), req.url().path()),
        otel.kind = "Client",
        http.request.method = method,
        server.address = req.url().domain(),
        server.port = req.url().port(),
        url.full = req.url().as_str(),
    );
    let _enter = span.enter();

    let mut header = HeaderMap::new();
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&span.context(), &mut HeaderInjector(&mut header))
    });
    let mut builder = builder;
    for (key, value) in header.iter() {
        builder = builder.header(key, value);
    }

    let start = std::time::Instant::now();
    let result = builder.send().await;
    let latency_ms = start.elapsed().as_millis();

    match &result {
        Ok(response) => {
            let response_status = response.status().as_u16() as i64;
            span.set_attribute("http.response.status_code", response_status);
            if response_status >= 400 {
                span.set_status(Status::Error {
                    description: format!("HTTP error: {}", response_status).into(),
                });
            }
            info!(
                http.request.method = method,
                http.response.latency_ms = latency_ms,
                http.response.status_code = response_status,
                url.path = path,
                url.query = query,
                "[{}] {} - {}",
                response_status,
                req.method(),
                req.url()
            );
        }
        Err(err) => {
            info!(
                error.message = err.to_string(),
                http.request.method = method,
                http.response.latency_ms = latency_ms,
                url.path = path,
                url.query = query,
                "[error] {} - {}",
                req.method(),
                req.url()
            );
            span.set_status(Status::Error {
                description: err.to_string().into(),
            });
        }
    }
    result
}

struct HeaderInjector<'a>(&'a mut http::HeaderMap);

impl Injector for HeaderInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(name) = http::header::HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(val) = http::header::HeaderValue::from_str(&value) {
                self.0.insert(name, val);
            }
        }
    }
}

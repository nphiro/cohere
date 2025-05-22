use axum::http;
use http::HeaderMap;
use opentelemetry::{global, propagation::Injector, trace::Status};
use reqwest::{Error, RequestBuilder, Response};
use tracing::error;
use tracing_opentelemetry::OpenTelemetrySpanExt;

macro_rules! dyn_event {
    ($lvl:ident, $($arg:tt)+) => {
        match $lvl {
            ::tracing::Level::TRACE => ::tracing::trace!($($arg)+),
            ::tracing::Level::DEBUG => ::tracing::debug!($($arg)+),
            ::tracing::Level::INFO => ::tracing::info!($($arg)+),
            ::tracing::Level::WARN => ::tracing::warn!($($arg)+),
            ::tracing::Level::ERROR => ::tracing::error!($($arg)+),
        }
    };
}

pub async fn send_http(
    builder: RequestBuilder,
    route_template: Option<&str>,
) -> Result<Response, Error> {
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
        otel.name = format!(
            "{} {}",
            req.method(),
            route_template.unwrap_or(req.url().path())
        ),
        otel.kind = "Client",
        http.request.method = method,
        server.address = req.url().domain(),
        server.port = req.url().port(),
        url.full = req.url().as_str(),
        url.template = route_template,
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
            let mut lvl = tracing::Level::INFO;
            let response_status = response.status().as_u16() as i64;
            span.set_attribute("http.response.status_code", response_status);
            if response_status >= 400 {
                span.set_status(Status::Error {
                    description: format!("HTTP error: {}", response_status).into(),
                });
                lvl = tracing::Level::ERROR;
            }
            dyn_event!(
                lvl,
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
            span.set_status(Status::Error {
                description: err.to_string().into(),
            });
            error!(
                error.message = err.to_string(),
                http.request.method = method,
                http.response.latency_ms = latency_ms,
                url.path = path,
                url.query = query,
                "[error] {} - {}",
                req.method(),
                req.url(),
            );
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

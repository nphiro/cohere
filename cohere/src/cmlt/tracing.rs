use opentelemetry::global;
use opentelemetry_sdk::{Resource, propagation::TraceContextPropagator, trace as sdktrace};
use opentelemetry_stdout::SpanExporter;

/// initializes OpenTelemetry tracing
pub fn init() -> sdktrace::SdkTracerProvider {
    global::set_text_map_propagator(TraceContextPropagator::new());

    let provider = sdktrace::SdkTracerProvider::builder()
        .with_simple_exporter(SpanExporter::default())
        .with_resource(Resource::builder().with_service_name("cohere").build())
        .build();

    global::set_tracer_provider(provider.clone());
    provider
}

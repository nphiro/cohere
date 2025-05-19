use opentelemetry::{
    Context, KeyValue,
    baggage::BaggageExt,
    global::{self, ObjectSafeSpan},
    propagation::TextMapCompositePropagator,
    trace::TracerProvider,
};
use opentelemetry_sdk::{
    Resource,
    error::OTelSdkResult,
    propagation::{BaggagePropagator, TraceContextPropagator},
    trace::{SdkTracerProvider, Span, SpanProcessor},
};
use opentelemetry_stdout::SpanExporter;
use tracing_subscriber::{Registry, layer::SubscriberExt};

use super::logging::LogLayer;

pub fn init(org: &str, project: &str) -> anyhow::Result<SdkTracerProvider> {
    let composite_propagator = TextMapCompositePropagator::new(vec![
        Box::new(BaggagePropagator::new()),
        Box::new(TraceContextPropagator::new()),
    ]);
    global::set_text_map_propagator(composite_propagator);

    let provider = SdkTracerProvider::builder()
        .with_span_processor(EnrichWithBaggageSpanProcessor)
        .with_simple_exporter(SpanExporter::default())
        .with_resource(
            Resource::builder()
                .with_service_name(project.to_owned())
                .build(),
        )
        .build();

    let telemetry =
        tracing_opentelemetry::layer().with_tracer(provider.tracer(format!("{}/{}", org, project)));

    let subscriber = Registry::default().with(telemetry).with(LogLayer);

    tracing::subscriber::set_global_default(subscriber)?;

    global::set_tracer_provider(provider.clone());
    Ok(provider)
}

#[derive(Debug)]
struct EnrichWithBaggageSpanProcessor;
impl SpanProcessor for EnrichWithBaggageSpanProcessor {
    fn on_start(&self, span: &mut Span, cx: &Context) {
        for (kk, vv) in cx.baggage().iter() {
            span.set_attribute(KeyValue::new(kk.clone(), vv.0.clone()));
        }
    }

    fn on_end(&self, _span: opentelemetry_sdk::trace::SpanData) {}

    fn force_flush(&self) -> OTelSdkResult {
        Ok(())
    }

    fn shutdown(&self) -> OTelSdkResult {
        Ok(())
    }
}

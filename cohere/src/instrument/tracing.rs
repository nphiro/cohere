use std::io::Write;

use opentelemetry::global::{self, ObjectSafeSpan};
use opentelemetry::{
    Context, KeyValue, baggage::BaggageExt, propagation::TextMapCompositePropagator,
    trace::TracerProvider,
};
use opentelemetry_sdk::{
    Resource,
    error::OTelSdkResult,
    propagation::{BaggagePropagator, TraceContextPropagator},
    trace::{SdkTracerProvider, Span, SpanProcessor},
};
use opentelemetry_stdout::SpanExporter;
use rustc_hash::FxHashMap;
use tracing::field::Visit;
use tracing_subscriber::Registry;
use tracing_subscriber::{Layer, layer::SubscriberExt};

pub fn init(org: &str, project: &str) -> anyhow::Result<SdkTracerProvider> {
    let baggage_propagator = BaggagePropagator::new();
    let trace_context_propagator = TraceContextPropagator::new();
    let composite_propagator = TextMapCompositePropagator::new(vec![
        Box::new(baggage_propagator),
        Box::new(trace_context_propagator),
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

    let subscriber = Registry::default().with(telemetry).with(LogEventLayer);

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

struct LogEventLayer;
impl<S> Layer<S> for LogEventLayer
where
    S: tracing::Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);

        let lvl = event.metadata().level().to_string();

        if let Some(span) = ctx.event_span(event) {
            let opt_span_id = span
                .extensions()
                .get::<tracing_opentelemetry::OtelData>()
                .and_then(|otd| otd.builder.span_id);
            let opt_trace_id = span.scope().last().and_then(|root_span| {
                root_span
                    .extensions()
                    .get::<tracing_opentelemetry::OtelData>()
                    .and_then(|otd| otd.builder.trace_id)
            });

            if let Some((trace_id, span_id)) = opt_trace_id.zip(opt_span_id) {
                visitor.print(lvl, trace_id.to_string(), span_id.to_string());
                return;
            }
        }
        visitor.print(lvl, "".into(), "".into());
    }
}

#[derive(Default)]
struct EventVisitor {
    msg: String,
    attrs: FxHashMap<String, String>,
}

#[derive(serde::Serialize)]
struct Event {
    message: String,
    level: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    trace_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    span_id: String,
    #[serde(skip_serializing_if = "FxHashMap::is_empty")]
    attributes: FxHashMap<String, String>,
}

impl EventVisitor {
    fn print(self, level: String, trace_id: String, span_id: String) {
        let json = Event {
            message: self.msg,
            level,
            trace_id,
            span_id,
            attributes: self.attrs,
        };
        let mut buffer = Vec::with_capacity(512);
        if serde_json::to_writer(&mut buffer, &json).is_ok() {
            let mut stdout = std::io::stdout().lock();
            let _ = stdout.write_all(&buffer);
            let _ = stdout.write_all(b"\n");
        } else {
            println!("{}", serde_json::to_string(&json).unwrap());
        }
    }
}

impl Visit for EventVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.msg = format!("{:?}", value);
        } else {
            self.attrs
                .insert(field.name().to_string(), format!("{:?}", value));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.msg = value.into();
        } else {
            self.attrs
                .insert(field.name().to_string(), value.to_string());
        }
    }
}

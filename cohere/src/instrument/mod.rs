mod logging;
mod tracing;

use opentelemetry_sdk::trace as sdktrace;

pub struct InstrumentGuard {
    tracer_provider: sdktrace::SdkTracerProvider,
}

/// Return a guard that will flush instrumentation data when it is dropped.
pub fn init(org: &str, project: &str) -> anyhow::Result<InstrumentGuard> {
    let provider = tracing::init(org, project)?;
    Ok(InstrumentGuard {
        tracer_provider: provider,
    })
}

impl Drop for InstrumentGuard {
    fn drop(&mut self) {
        if let Err(err) = self.tracer_provider.shutdown() {
            println!("Error shutting down tracer provider: {:?}", err);
        }
    }
}

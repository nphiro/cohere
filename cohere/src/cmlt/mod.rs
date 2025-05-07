mod tracing;

use opentelemetry_sdk::trace as sdktrace;

pub struct Handler {
    tracer_provider: sdktrace::SdkTracerProvider,
}

/// Initializes logging and returns an object that will
/// automatically flush buffered data when it goes out of scope.
pub fn init() -> Handler {
    let provider = tracing::init();
    Handler {
        tracer_provider: provider,
    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        if let Err(err) = self.tracer_provider.shutdown() {
            println!("Error shutting down tracer provider: {:?}", err);
        }
    }
}

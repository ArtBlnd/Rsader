fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        tracing_subscriber::fmt()
            .with_writer(
                tracing_subscriber_wasm::MakeConsoleWriter::default()
                    .map_trace_level_to(tracing::Level::TRACE),
            )
            .without_time()
            .init();
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }
}

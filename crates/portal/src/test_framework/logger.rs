use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize the logger once across all threads
///
/// This function is safe to call multiple times; the logger will only be
/// initialized on the first call.
pub fn init_logger() {
    INIT.call_once(|| {
        env_logger::init();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_init() {
        // First call should succeed
        init_logger();

        // Second call should be a no-op
        init_logger();

        // If we get here without panicking, the test passes
    }
}

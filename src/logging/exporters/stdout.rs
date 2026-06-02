use tracing::info;

pub fn export(
    message: &str,
) {
    info!(
        export = message,
        "stdout exporter"
    );
}

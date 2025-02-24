pub fn format_bytes(bytes: f64) -> String {
    if bytes > 1_073_741_824_f64 {
        format!("{:.2}GB", bytes / 1_073_741_824_f64)
    } else if bytes > 1_048_576_f64 {
        format!("{:.2}MB", bytes / 1_048_576_f64)
    } else if bytes > 1_024_f64 {
        format!("{:.2}KB", bytes / 1_024_f64)
    } else {
        format!("{:.2}B", bytes)
    }
}

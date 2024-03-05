pub fn format_ram(bytes: u64) -> String {
    if bytes > 1_073_741_824 {
        format!("{:.2}GB", bytes as f64 / 1_073_741_824_f64)
    } else if bytes > 1_048_576 {
        format!("{:.2}MB", bytes as f64 / 1_048_576_f64)
    } else if bytes > 1_024 {
        format!("{:.2}KB", bytes as f64 / 1_024_f64)
    } else {
        format!("{:.2}B", bytes)
    }
}

pub fn format_vram(mega_bytes: u64) -> String {
    if mega_bytes > 1_024 {
        format!("{:.2}GB", mega_bytes as f64 / 1024_f64)
    } else {
        format!("{}MB", mega_bytes)
    }
}

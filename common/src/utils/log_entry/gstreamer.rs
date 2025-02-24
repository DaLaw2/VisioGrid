use glib::error::Error as GError;
use gstreamer::StateChangeError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GStreamerEntry {
    #[error("GStreamer initialization failed: {0}")]
    InitializeError(GError),
    #[error("Failed to create GStreamer pipeline: {0}")]
    CreatePipelineError(GError),
    #[error("Failed to get GStreamer bus")]
    GetBusError,
    #[error("Failed to set pipeline status: {0}")]
    PipelineSetStateError(StateChangeError),
    #[error("GStreamer internal error: {0}")]
    InternalError(GError),
}

impl From<GStreamerEntry> for String {
    #[inline(always)]
    fn from(value: GStreamerEntry) -> Self {
        value.to_string()
    }
}

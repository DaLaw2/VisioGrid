use crate::management::utils::inference_argument::InferenceArgument;
use crate::utils::logging::*;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command as AsyncCommand;

pub struct InferenceManager;

impl InferenceManager {
    pub async fn ultralytics_inference_image(inference_argument: InferenceArgument,
                                             model_path: PathBuf, image_path: PathBuf) -> Result<(), LogEntry>
    {
        #[cfg(target_os = "windows")]
        let python = "python";
        #[cfg(target_os = "linux")]
        let python = "python3";
        let save_folder = PathBuf::from("./Result");
        let mut process = AsyncCommand::new(python)
            .arg("Script/ultralytics/picture_inference.py")
            .arg(inference_argument.detect_mode.to_string())
            .arg(model_path)
            .arg(image_path)
            .arg(save_folder)
            .arg(inference_argument.imgsz.to_string())
            .arg(inference_argument.conf.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| error_entry!(SystemEntry::ChildProcessError(err.to_string())))?;
        let status = process.wait().await
            .map_err(|err| error_entry!(SystemEntry::ChildProcessError(err.to_string())))?;
        if !status.success() {
            let err = format!("Process exit with code: {}", status.code().unwrap_or(-1));
            Err(error_entry!(SystemEntry::ChildProcessError(err)))?
        }
        Ok(())
    }

    pub async fn ultralytics_inference_video(inference_argument: InferenceArgument,
                                             model_path: PathBuf, video_path: PathBuf) -> Result<(), LogEntry>
    {
        #[cfg(target_os = "windows")]
        let python = "python";
        #[cfg(target_os = "linux")]
        let python = "python3";
        let save_folder = PathBuf::from("./Result");
        let mut process = AsyncCommand::new(python)
            .arg("Script/ultralytics/video_inference.py")
            .arg(inference_argument.detect_mode.to_string())
            .arg(model_path)
            .arg(video_path)
            .arg(save_folder)
            .arg(inference_argument.imgsz.to_string())
            .arg(inference_argument.conf.to_string())
            .arg(inference_argument.batch.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| error_entry!(SystemEntry::ChildProcessError(err.to_string())))?;
        let status = process.wait().await
            .map_err(|err| error_entry!(SystemEntry::ChildProcessError(err.to_string())))?;
        if !status.success() {
            let err = format!("Process exit with code: {}", status.code().unwrap_or(-1));
            Err(error_entry!(SystemEntry::ChildProcessError(err)))?
        }
        Ok(())
    }

    #[allow(unused_variables)]
    pub async fn yolov4_inference_picture(inference_argument: InferenceArgument,
                                          model_path: PathBuf, video_path: PathBuf) -> Result<(), LogEntry>
    {
        Ok(())
    }

    #[allow(unused_variables)]
    pub async fn yolov4_inference_video(inference_argument: InferenceArgument,
                                        model_path: PathBuf, video_path: PathBuf) -> Result<(), LogEntry>
    {
        Ok(())
    }


    #[allow(unused_variables)]
    pub async fn yolov7_inference_picture(inference_argument: InferenceArgument,
                                          model_path: PathBuf, video_path: PathBuf) -> Result<(), LogEntry>
    {
        Ok(())
    }


    #[allow(unused_variables)]
    pub async fn yolov7_inference_video(inference_argument: InferenceArgument,
                                        model_path: PathBuf, video_path: PathBuf) -> Result<(), LogEntry>
    {
        Ok(())
    }
}

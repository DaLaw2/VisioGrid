import argparse
import logging
import sys
from pathlib import Path

import ffmpeg
from ultralytics.models.yolo.detect import DetectionPredictor
from ultralytics.trackers import register_tracker
from ultralytics.utils import LOGGER
from ultralytics.utils import callbacks, MACOS, WINDOWS


class VideoInference:
    def __init__(self, mode: str, model_path: Path, video_path: Path, save_path: Path, **kwargs):
        self.mode = mode
        self.model_path = model_path
        self.video_path = video_path
        self.save_path = save_path
        self.args = kwargs
        if not kwargs.get("verbose", False):
            LOGGER.setLevel(logging.NOTSET)

    def get_codec_name(self, video_path: Path) -> str:
        """
        Retrieve the codec name of a video using ffmpeg.

        :param video_path: Path to the video file.
        :return: Codec name as a string.
        """
        try:
            probe = ffmpeg.probe(str(video_path))
            streams = [stream for stream in probe['streams'] if stream['codec_type'] == 'video']
            if not streams:
                self.panic(f"No video streams found in {video_path}")
            codec_name = streams[0]['codec_name']
            return codec_name
        except Exception as e:
            self.panic(f"Error getting codec name for {video_path}: {e}")

    def yolo_predict(self):
        """
        Perform YOLO prediction on the input video and save the results.
        """
        try:
            default_args = {
                'batch': 16,
                'conf': 0.25,
                'imgsz': 640,
                'mode': 'predict',
                'model': self.model_path,
                'save': True,
                'task': 'detect',
            }
            args = {**default_args, **self.args}
            callback = callbacks.get_default_callbacks()
            predictor = DetectionPredictor(overrides=args, _callbacks=callback)
            predictor.setup_model(model=self.model_path)
            predictor.save_dir = self.save_path
            predictor.predict_cli(source=self.video_path)
        except Exception as e:
            self.panic(f"Error during YOLO prediction: {e}")

    def yolo_track(self):
        """
        Perform YOLO tracking on the input video and save the results.
        """
        try:
            default_args = {
                'batch': 1,
                'conf': 0.1,
                'imgsz': 640,
                'mode': 'track',
                'model': self.model_path,
                'save': True,
                'task': 'detect',
            }
            args = {**default_args, **self.args}
            callback = callbacks.get_default_callbacks()
            predictor = DetectionPredictor(overrides=args, _callbacks=callback)
            register_tracker(predictor, False)
            predictor.setup_model(model=self.model_path)
            predictor.save_dir = self.save_path
            predictor.predict_cli(source=self.video_path)
        except Exception as e:
            self.panic(f"Error during YOLO tracking: {e}")

    @staticmethod
    def platform_specific_format() -> str:
        """
        Determine the platform-specific video format.

        :return: File extension as a string.
        """
        return "mp4" if MACOS else "avi" if WINDOWS else "avi"

    def transform(self, input_video: Path, temp_video: Path, video_codec: str):
        """
        Convert a video to the target format using ffmpeg-python.

        :param input_video: Path to the input video file to be converted.
        :param temp_video: Path to save the temporary converted video file.
        :param video_codec: Target video codec name.
        """
        try:
            verbose: bool = not self.args.get("verbose", False)
            (
                ffmpeg
                .input(str(input_video))
                .output(str(temp_video), vcodec=video_codec, **{'strict': '-2'})
                .overwrite_output()
                .run(quiet=verbose)
            )
        except ffmpeg.Error as e:
            self.panic(f"Error transforming video {input_video} to {temp_video}: {e}")

    @staticmethod
    def panic(*args, **kwargs):
        """
        Print an error message to stderr and exit the program.

        :param args: Arguments to print.
        :param kwargs: Keyword arguments to print.
        """
        print(*args, file=sys.stderr, **kwargs)
        sys.exit(1)

    def inference(self):
        """
        Inference the video by performing YOLO tracking/prediction and handling codec transformations.
        """
        try:
            video_filename = self.video_path.stem
            video_suffix = self.video_path.suffix.lower().replace(".", "")
            video_codec = self.get_codec_name(self.video_path)

            if self.mode == "predict":
                self.yolo_predict()
            elif mode == "track":
                self.yolo_track()
            else:
                self.panic("Unexpected error: Invalid detect mode.")

            platform_extension = self.platform_specific_format()
            predicted_video = self.save_path / f"{video_filename}.{platform_extension}"
            if not predicted_video.exists():
                self.panic("Unexpected error: Predicted video does not exist.")

            if not MACOS:
                temp_output_video = self.save_path / f"{video_filename}_temp.{video_suffix}"

                self.transform(predicted_video, temp_output_video, video_codec)

                predicted_video.unlink()
                final_output_video = self.save_path / f"{video_filename}.{video_suffix}"
                temp_output_video.rename(final_output_video)
            else:
                final_output_video = self.save_path / f"{video_filename}.{video_suffix}"
                predicted_video.rename(final_output_video)
        except Exception as e:
            self.panic(f"An unexpected error occurred: {e}")


if __name__ == "__main__":
    try:
        parser = argparse.ArgumentParser(description='Video Inference Script',
                                         usage='%(prog)s mode model video save imgsz conf batch verbose',
                                         formatter_class=argparse.RawTextHelpFormatter)
        parser.add_argument('mode', type=str, choices=['predict', 'track'],
                            help="Mode of operation: 'predict' or 'track'")
        parser.add_argument('model_path', type=Path, help='Path to the model')
        parser.add_argument('video_path', type=Path, help='Path to the input video')
        parser.add_argument('save_path', type=Path, help='Path to save the output')
        parser.add_argument('imgsz', type=int, help='Image size for processing')
        parser.add_argument('conf', type=float, help='Confidence threshold')
        parser.add_argument('batch', type=int, help='Batch size')
        parser.add_argument('--verbose', action='store_true', help='Enable verbose logging')

        args = parser.parse_args()
        mode = args.mode
        model_path = args.model_path
        video_path = args.video_path
        save_path = args.save_path
        other_args = {'imgsz': args.imgsz, 'conf': args.conf, "batch": args.batch, 'verbose': args.verbose}

        video_inference = VideoInference(mode, model_path, video_path, save_path, **other_args)
        video_inference.inference()
    except Exception as e:
        print(f"An unexpected error occurred: {e}", file=sys.stderr)
        sys.exit(1)

import argparse
import logging
import sys
from pathlib import Path

from ultralytics.models.yolo.detect import DetectionPredictor
from ultralytics.utils import LOGGER
from ultralytics.utils import callbacks


class PictureInference:
    def __init__(self, model_path: Path, picture_path: Path, save_path: Path, **kwargs):
        self.model_path = model_path
        self.picture_path = picture_path
        self.save_path = save_path
        self.args = kwargs
        if not kwargs.get("verbose", False):
            LOGGER.setLevel(logging.NOTSET)

    def yolo_predict(self):
        """
        Perform YOLO prediction on the input picture and save the results.
        """
        try:
            default_args = {
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
            predictor.predict_cli(source=self.picture_path)
        except Exception as e:
            self.panic(f"Error during YOLO prediction: {e}")

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
        Inference the picture by performing YOLO tracking/prediction and handling codec transformations.
        """
        try:
            picture_filename = self.picture_path.stem
            picture_suffix = self.picture_path.suffix.lower().replace(".", "")

            self.yolo_predict()

            predicted_picture = self.save_path / f"{picture_filename}.{picture_suffix}"
            if not predicted_picture.exists():
                self.panic("Unexpected error: Predicted picture does not exist.")
        except Exception as e:
            self.panic(f"An unexpected error occurred: {e}")


if __name__ == "__main__":
    try:
        parser = argparse.ArgumentParser(description='picture Inference Script',
                                         usage='%(prog)s mode model picture save imgsz conf verbose',
                                         formatter_class=argparse.RawTextHelpFormatter)
        parser.add_argument('model_path', type=Path, help='Path to the model')
        parser.add_argument('picture_path', type=Path, help='Path to the input picture')
        parser.add_argument('save_path', type=Path, help='Path to save the output')
        parser.add_argument('imgsz', type=int, help='Image size for processing')
        parser.add_argument('conf', type=float, help='Confidence threshold')
        parser.add_argument('--verbose', action='store_true', help='Enable verbose logging')

        args = parser.parse_args()
        model_path = args.model_path
        picture_path = args.picture_path
        save_path = args.save_path
        other_args = {'imgsz': args.imgsz, 'conf': args.conf, 'verbose': args.verbose}
        picture_inference = PictureInference(model_path, picture_path, save_path, **other_args)
        picture_inference.inference()
    except Exception as e:
        print(f"An unexpected error occurred: {e}", file=sys.stderr)
        sys.exit(1)

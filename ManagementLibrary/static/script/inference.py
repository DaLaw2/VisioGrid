import sys
import json
from ultralytics import YOLO

class BoundingBox:
    def __init__(self, box: list, name: str, confidences: float):
        self.xmin, self.ymin, self.xmax, self.ymax = box
        self.xmin = int(self.xmin)
        self.xmax = int(self.xmax)
        self.ymin = int(self.ymin)
        self.ymax = int(self.ymax)
        self.name = name
        self.confidences = confidences

    def to_dict(self):
        return {
            "xmin": self.xmin,
            "xmax": self.xmax,
            "ymin": self.ymin,
            "ymax": self.ymax,
            "name": self.name,
            "confidences": self.confidences
        }

def panic(*args, **kwargs):
    print(*args, file=sys.stderr, **kwargs)
    sys.exit(1)

if __name__ == "__main__":
    if len(sys.argv) != 3:
        panic("Unexpected argument.")
    model_path: str = sys.argv[1]
    media_path: str = sys.argv[2]
    try:
        model: YOLO = YOLO(model_path)
        try:
            result: list = model.predict(media_path, verbose=False)
            bounding_boxs: list = []
            boxes: list = result[0].boxes.xyxy.tolist()
            names: list = result[0].names
            classes: list = result[0].boxes.cls.tolist()
            confidences: list = result[0].boxes.conf.tolist()
            for box, cls, conf in zip(boxes, classes, confidences):
                confidence: float = conf
                name: str = names[int(cls)]
                bounding_boxs.append(BoundingBox(box, name, confidence).to_dict())
            print(json.dumps(bounding_boxs))
        except AttributeError as err:
            panic("Unable to inference media.")
        except IndexError as err:
            panic("Unknown error.")
    except FileNotFoundError as err:
        panic("Model does not exist.")
    except ValueError as err:
        panic("Unable to parse model file.")
    except ImportError as err:
        panic("Model file is not support.")

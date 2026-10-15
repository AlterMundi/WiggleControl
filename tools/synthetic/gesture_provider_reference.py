#!/usr/bin/env python3
"""Retain full MediaPipe CPU results on explicit image files, never a camera.

These results qualify conversion/preprocessing, not independent label truth.
Public tutorial images may overlap the provider's training/evaluation data.
"""
import argparse
import hashlib
import json
from pathlib import Path

import mediapipe as mp

from export_gesture_classifier import BUNDLE_SHA256


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('bundle', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('images', type=Path, nargs='+')
    args = parser.parse_args()
    if args.output.exists() or not 1 <= len(args.images) <= 256:
        parser.error('New output file and1..256 explicit images required')
    if mp.__version__ != '1.1.1' or hashlib.sha256(args.bundle.read_bytes()).hexdigest() != BUNDLE_SHA256:
        parser.error('Expected mediapipe1.1.1 and the pinned model bundle')
    options = mp.tasks.vision.GestureRecognizerOptions(
        base_options=mp.tasks.BaseOptions(model_asset_path=str(args.bundle.resolve()),
                                         delegate=mp.tasks.BaseOptions.Delegate.CPU),
        running_mode=mp.tasks.vision.RunningMode.IMAGE, num_hands=2)
    samples, hands = [], []
    with mp.tasks.vision.GestureRecognizer.create_from_options(options) as model:
        for path in args.images:
            image = mp.Image.create_from_file(str(path.resolve()))
            if image.width > 4096 or image.height > 4096:
                raise ValueError('Image exceeds4096 pixels per axis')
            result = model.recognize(image)
            samples.append(dict(name=path.name, sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                                detected_hands=len(result.hand_landmarks)))
            for index, (points, world, handedness, gestures) in enumerate(zip(
                    result.hand_landmarks, result.hand_world_landmarks,
                    result.handedness, result.gestures)):
                category = gestures[0]
                hands.append(dict(name=path.name+'#'+str(index), width=image.width, height=image.height,
                    landmarks=[[v.x, v.y, v.z] for v in points],
                    world_landmarks=[[v.x, v.y, v.z] for v in world],
                    handedness=[dict(name=v.category_name, score=v.score) for v in handedness],
                    category=category.category_name, score=category.score))
    args.output.write_text(json.dumps(dict(schema='wigglecontrol.provider-gesture-reference/v1',
        mediapipe_version=mp.__version__, bundle_sha256=BUNDLE_SHA256,
        image_rotation_degrees=0, samples=samples, hands=hands,
        camera_opened=False, system_input=False,
        limits='Provider predictions, not independent human intent or held-out accuracy labels'), indent=2)+'\n')
    print(json.dumps(dict(images=len(samples), hands=len(hands), camera_opened=False)))


if __name__ == '__main__':
    main()

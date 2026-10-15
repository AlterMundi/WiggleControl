#!/usr/bin/env python3
"""Pinned CPU Tasks comparison on existing synthetic images; no camera or input."""
import argparse
import hashlib
import json
from pathlib import Path
import time

import mediapipe as mp


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('manifest', type=Path)
    p.add_argument('model', type=Path)
    p.add_argument('--expected-sha256', required=True)
    args = p.parse_args()
    if mp.__version__ != '1.1.1':
        p.error('This comparison requires mediapipe==1.1.1')
    if hashlib.sha256(args.model.read_bytes()).hexdigest() != args.expected_sha256:
        p.error('Model hash mismatch')
    if args.manifest.stat().st_size > 1_048_576:
        p.error('Manifest exceeds1MiB')
    sequence = json.loads(args.manifest.read_text())
    frames = sequence['frames']
    if sequence['version'] != 1 or not 1 <= len(frames) <= 10_000:
        p.error('Expected sequence version1 with1..10000 frames')
    root = args.manifest.resolve().parent
    last = None
    # This offline comparison uses the provider's native CPU API. It does not
    # replace the Rust runtime or pretend detector array positions are identities.
    options = mp.tasks.vision.HandLandmarkerOptions(
        base_options=mp.tasks.BaseOptions(model_asset_path=str(args.model.resolve()),
                                         delegate=mp.tasks.BaseOptions.Delegate.CPU),
        running_mode=mp.tasks.vision.RunningMode.VIDEO, num_hands=2,
        min_hand_detection_confidence=0.5, min_hand_presence_confidence=0.5,
        min_tracking_confidence=0.5)
    with mp.tasks.vision.HandLandmarker.create_from_options(options) as model:
        for index, frame in enumerate(frames):
            source_us = frame['source_us']
            ms = source_us // 1000
            if last is not None and ms <= last:
                p.error('Tasks requires strictly increasing millisecond source times')
            last = ms
            started = time.monotonic()
            name = frame['image']
            if name is None:
                p.error('Missing-image semantics require a separately qualified Tasks reset; not silently skipped')
            path = (root/name).resolve()
            if root not in path.parents:
                p.error('Image path escapes manifest root')
            image = mp.Image.create_from_file(str(path))
            if image.width > 4096 or image.height > 4096:
                p.error('Image exceeds4096 pixels per axis')
            result = model.detect_for_video(image, ms)
            hands = []
            for i, (points, world, side) in enumerate(zip(
                    result.hand_landmarks, result.hand_world_landmarks, result.handedness)):
                gap = sum((getattr(world[4], a)-getattr(world[8], a))**2 for a in ('x', 'y', 'z'))**0.5
                hands.append(dict(array_index=i, stable_id_available=False,
                    landmarks=[[v.x*image.width, v.y*image.height] for v in points],
                    world_landmarks=[[v.x,v.y,v.z] for v in world], world_pinch_distance_m=gap,
                    handedness=[dict(name=s.category_name, score=s.score) for s in side]))
            print(json.dumps(dict(schema='wigglecontrol.sequence-frame/v1', frame=index,
                source_us=source_us, image=name, camera_opened=False, system_input=False,
                backend='mediapipe-1.1.1-cpu', offline_now_equals_source=True,
                processing_ms=(time.monotonic()-started)*1000, ambiguous=None,
                ambiguity_available=False, hands=hands, contacts=None,
                gesture_meaning_evaluated=False), allow_nan=False), flush=True)


if __name__ == '__main__':
    main()

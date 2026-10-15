"""Bounded, asset-bound research articulations; not certified human joint limits."""
import hashlib
import json
import math


DEFAULT_ANGLES = {
    'r_index1': 0.5780913844963557, 'r_index2': 0.9080790178234996,
    'r_index3': 0.6839674325177723, 'r_thumb0': 0.4828462042455118,
    'r_thumb1': -0.06196422713524768, 'r_thumb2': 0.21760893089706515,
    'r_thumb3': 0.0,
}
LIMITS = {'r_index1': [0, 1.0], 'r_index2': [0, 1.2], 'r_index3': [0, 1.0],
          'r_thumb0': [-1.0, 1.0], 'r_thumb1': [-0.5, 0.5],
          'r_thumb2': [0, 1.0], 'r_thumb3': [0, 1.0]}


def load(path, asset_sha256):
    if path.stat().st_size > 8192:
        raise ValueError('Pinch pose exceeds8KiB')
    content = path.read_bytes()
    data = json.loads(content)
    if (not isinstance(data, dict)
            or set(data) != {'version', 'asset_sha256', 'joint_angles_rad'}
            or type(data['version']) is not int or data['version'] != 1
            or data['asset_sha256'] != asset_sha256):
        raise ValueError('Expected version1 pinch pose for this exact asset')
    angles = data['joint_angles_rad']
    if not isinstance(angles, dict) or set(angles) != set(LIMITS):
        raise ValueError('Pinch pose must specify exactly the seven supported joints')
    for name, value in angles.items():
        low, high = LIMITS[name]
        if type(value) not in (int, float) or not math.isfinite(value) or not low <= value <= high:
            raise ValueError('Pinch pose exceeds retained forward recipe bounds: '+name)
    return angles, hashlib.sha256(content).hexdigest()

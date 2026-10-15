#!/usr/bin/env python3
"""Summarize retained closure geometry and predictions without treating either as intent."""
import argparse
import hashlib
import json
import math
from pathlib import Path


def summarize(path):
    data = json.loads(path.read_text())
    if data['camera_opened'] or data['system_input']:
        raise ValueError('Expected an offline, input-free experiment')
    labels, predictions = data['reference']['frames'], data['predictions']
    if not labels or len(labels) != len(predictions):
        raise ValueError('Missing reference or prediction frames')
    for i, (label, prediction) in enumerate(zip(labels, predictions)):
        if label['frame'] != i or prediction['frame'] != i:
            raise ValueError('Frame order differs from the reference')
        if label['source_us'] != prediction['source_us']:
            raise ValueError('Source time differs from the reference')
    maximum = max(f['closure_interpolation'] for f in labels)
    if maximum <= 0:
        raise ValueError('Expected a nonzero articulated closure')
    near = [i for i, f in enumerate(labels)
            if math.isclose(f['closure_interpolation'], maximum, abs_tol=1e-9)]
    opened = [i for i, f in enumerate(labels) if f['closure_interpolation'] == 0]
    active = [i for i, p in enumerate(predictions) if p['contacts']]
    intervals = []
    for i in active:
        if not intervals or i != intervals[-1][-1]+1:
            intervals.append([i])
        else:
            intervals[-1].append(i)
    def model_gaps(indices):
        return [h['pinch_distance_m']*1000 for i in indices
                for h in predictions[i]['hands'] if h.get('pinch_distance_m') is not None]
    def span(values):
        return [min(values), max(values)] if values else None
    # A static maximum-closure hold supplies a stationarity reference. This is
    # spread about its mean, not absolute target error or physical accuracy.
    contact_points = [(p['x']*1920, p['y']*1080) for i in near
                      for p in predictions[i]['contacts']]
    if contact_points:
        mx = sum(p[0] for p in contact_points)/len(contact_points)
        my = sum(p[1] for p in contact_points)/len(contact_points)
        rms = math.sqrt(sum((x-mx)**2+(y-my)**2 for x, y in contact_points)/len(contact_points))
    else:
        rms = None
    return dict(view=data['reference'].get('closing_view') or 'reference',
        report_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
        total_frames=len(labels),
        detected_frames=sum(bool(p['hands']) for p in predictions),
        ambiguous_frames=sum(p['ambiguous'] for p in predictions),
        track_ids=sorted({h['id'] for p in predictions for h in p['hands']}),
        skin_in_frame=(all(f['skin_in_frame'] for f in labels)
                       if all('skin_in_frame' in f for f in labels) else None),
        skin_gap_mm=span([f['distal_skin']['surface_gap_m']*1000 for f in labels]),
        surface_intersection_frames=[i for i, f in enumerate(labels)
                                     if f['distal_skin']['surface_intersection']],
        contact_intervals_inclusive=[[v[0], v[-1]] for v in intervals],
        near_hold_frames=near,
        near_hold_with_contact=sum(bool(predictions[i]['contacts']) for i in near),
        near_hold_model_gap_mm=span(model_gaps(near)),
        near_contact_rms_about_mean_px=rms,
        spread_output_size=[1920, 1080],
        open_hold_frames=opened,
        open_hold_with_contact=sum(bool(predictions[i]['contacts']) for i in opened),
        open_hold_model_gap_mm=span(model_gaps(opened)))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reports', type=Path, nargs='+', help='Retained visible-motion results.json files')
    args = parser.parse_args()
    print(json.dumps(dict(schema='wigglecontrol.closure-views/v1',
        rows=[summarize(p) for p in args.reports],
        limits='Near/open holds follow renderer articulation, not model output. Selected skin gap, contact intent, anatomical limits and physical acceptance remain distinct.',
        camera_opened=False, system_input=False, live_config_changed=False), indent=2))


if __name__ == '__main__':
    main()

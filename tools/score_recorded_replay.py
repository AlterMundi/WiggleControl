#!/usr/bin/env python3
"""Measure recorded-source replay and explicit adjacent-frame appearance labels.

No inference, capture or input. Appearance is not physical contact or intent.
Sparse labels never become continuous holds; coalesced source frames remain scored.
"""
import argparse
import bisect
import hashlib
import json
from pathlib import Path
import statistics


def require(condition, message):
    if not condition:
        raise ValueError(message)


def validate(events):
    sources = [e for e in events if e['event'] == 'listening']
    require(len(sources) == 1 and sources[0].get('source_kind') ==
            'recorded_frame_republication', 'Expected explicit recorded source')
    end = events[-1]
    require(end['event'] == 'stop' and end.get('error') is None and
            end.get('worker_closed') is True and end.get('terminal_visible') is True,
            'Missing successful producer/model/view termination')
    require(not end['controller']['contacts'] and not end['controller']['button_down'],
            'Final input is held')
    require(all(e.get('camera_opened') is False and e.get('system_input') is False
                for e in events), 'Replay acquired camera or system input')
    captures = [e for e in events if e['event'] == 'capture']
    frames = [e for e in events if e['event'] == 'frame']
    require(captures and frames, 'No source or accepted observations')
    fields = {'source_us', 'sequence', 'segment', 'offset', 'length'}
    require(all(isinstance(e.get('recorded_frame'), dict) and
                fields == set(e['recorded_frame']) and
                all(type(n) is int and n >= 0 for n in e['recorded_frame'].values()) and
                0 < e['recorded_frame']['length'] <= 2*1024*1024
                for e in captures), 'Missing or invalid original recording provenance')
    by = {e['sequence']: e for e in captures}
    require(len(by) == len(captures), 'Duplicate publication identity')
    require(len(captures) == end['captured'] == end['issued'] + end['skipped'] and
            end['issued'] == len(frames) + end['expired'], 'Inconsistent source counts')
    for a, b in zip(captures, captures[1:]):
        require(a['sequence'] < b['sequence'] and a['source_us'] < b['source_us'] and
                b['capture_sequence'] == a['capture_sequence'] + 1 and
                a['recorded_frame']['source_us'] < b['recorded_frame']['source_us'],
                'Source publication or original ordinal/clock is not ordered')
    for a, b in zip(frames, frames[1:]):
        require(a['sequence'] < b['sequence'] and a['meaning_us'] < b['meaning_us'],
                'Accepted observations are not ordered')
    for frame in frames:
        require(all(type(frame[k]) is int and frame[k] >= 0 for k in
                    ('source_us', 'issued_us', 'received_us', 'now_us', 'meaning_us')),
                'Invalid host timestamp type')
        captured = by.get(frame['sequence'])
        require(captured is not None and frame['recorded_frame'] ==
                captured['recorded_frame'] and frame['capture_sequence'] ==
                captured['capture_sequence'], 'Accepted original provenance mismatch')
        require(frame['source_us'] == captured['source_us'] and frame['source_us'] <=
                frame['issued_us'] <= frame['received_us'] <= frame['now_us'] <=
                frame['meaning_us'], 'Invalid host clock or publication identity')
    return captures, frames, end


def appearance(events, captures, frames, annotations, max_gap_ms):
    require(annotations['schema'] == 'wigglecontrol.recorded-appearance/v1' and
            annotations.get('annotation_kind') in ('assistant_visual_review', 'human_visual_review')
            and bool(annotations.get('limits')), 'Explicit annotation kind and limits required')
    labels = {r['index']: r for r in annotations['frames']}
    require(len(labels) == len(annotations['frames']), 'Duplicate annotation index')
    allowed = {'open', 'apparent_tip_contact', 'uncertain'}
    require(all(type(n) is int and n >= 0 and r['appearance'] in allowed
                and type(r['source_us']) is int for n, r in labels.items()),
            'Invalid appearance label')
    by = {r['capture_sequence']: r for r in captures}
    accepted = {r['capture_sequence']: r for r in frames}
    samples = []
    for n, label in sorted(labels.items()):
        source = by.get(n)
        if source is not None:
            require(label['source_us'] == source['recorded_frame']['source_us'],
                    'Annotation identifies a different original frame time')
        samples.append(dict(index=n, appearance=label['appearance'],
                            status='outside_window' if source is None else
                            'accepted' if n in accepted else 'coalesced',
                            contact=bool(accepted[n]['controller']['contacts'])
                            if n in accepted else None))
    # Frame transitions have an earlier cycle-now stamp as well as meaning time.
    # Count their actual meaning once; keep independent watchdog ticks separately.
    frame_cycles = {r['now_us'] for r in frames}
    timeline = sorted([(r['meaning_us'], bool(r['controller']['contacts'])) for r in frames] +
                      [(r['now_us'], bool(r['controller']['contacts'])) for r in events
                       if r['event'] == 'intent_transition' and r['now_us'] not in frame_cycles])
    times = [t for t, _ in timeline]
    totals = dict(apparent_contact_us=0, contact_during_apparent_contact_us=0,
                  open_us=0, contact_during_open_us=0)
    held_segments = []
    pairs = 0
    for first, following in zip(captures, captures[1:]):
        a, b = labels.get(first['capture_sequence']), labels.get(following['capture_sequence'])
        if (a is None or b is None or a['appearance'] != b['appearance'] or
                a['appearance'] == 'uncertain' or
                b['source_us'] - a['source_us'] > max_gap_ms * 1000):
            continue
        pairs += 1
        closed = a['appearance'] == 'apparent_tip_contact'
        kind = 'apparent_contact' if closed else 'open'
        start, end = first['source_us'], following['source_us']
        cursor = bisect.bisect_right(times, start)
        held = timeline[cursor-1][1] if cursor else False
        while start < end:
            boundary = min(end, timeline[cursor][0]) if cursor < len(timeline) else end
            totals[kind+'_us'] += boundary-start
            if held:
                totals['contact_during_'+kind+'_us'] += boundary-start
                if closed and boundary > start:
                    if held_segments and held_segments[-1][1] == start:
                        held_segments[-1][1] = boundary
                    else:
                        held_segments.append([start, boundary])
            start = boundary
            if cursor < len(timeline) and timeline[cursor][0] <= start:
                held = timeline[cursor][1]
                cursor += 1
    return dict(annotation_kind=annotations['annotation_kind'], annotation_limits=annotations['limits'],
                samples=samples, scored_adjacent_pairs=pairs,
                wall_time_ms={k.removesuffix('_us')+'_ms': v/1000 for k, v in totals.items()},
                held_segments_in_reviewed_contact_intervals=len(held_segments),
                longest_reviewed_contact_segment_ms=max((b-a)/1000 for a, b in held_segments)
                    if held_segments else 0,
                limits='Only identically labelled adjacent source ordinals within the maximum '
                       'original-time gap. Interpolates appearance between those adjacent images; '
                       'includes coalesced publications and actual processing delay. No ownership, '
                       'physical contact, intent or OS delivery accuracy claim.')


def analyze(events, annotations=None, max_gap_ms=150):
    captures, frames, end = validate(events)
    ages = sorted((r['meaning_us']-r['source_us'])/1000 for r in frames)
    result = dict(schema='wigglecontrol.recorded-replay-score/v1', captured=len(captures),
                  accepted=len(frames), skipped=end['skipped'], expired=end['expired'],
                  accepted_hz=(len(frames)-1)*1e6/(frames[-1]['meaning_us']-frames[0]['meaning_us'])
                      if len(frames) > 1 else None,
                  publication_to_meaning_ms=dict(median=statistics.median(ages),
                      p95=ages[int((len(ages)-1)*.95)], maximum=max(ages)),
                  final_neutral=True, worker_closed=True, terminal_visible=True,
                  camera_opened=False, system_input=False,
                  limits='Recorded encoded publication to meaning, excluding original acquisition '
                         'and OS/application output. Unlabelled output counts are not accuracy.')
    if annotations is not None:
        require(0 < max_gap_ms <= 1000, 'Invalid annotation interpolation gap')
        result['appearance'] = appearance(events, captures, frames, annotations, max_gap_ms)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('journal', type=Path)
    parser.add_argument('--annotations', type=Path)
    parser.add_argument('--max-label-gap-ms', type=float, default=150)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    events = [json.loads(line) for line in args.journal.read_text().splitlines()]
    annotations = json.loads(args.annotations.read_text()) if args.annotations else None
    result = analyze(events, annotations, args.max_label_gap_ms)
    result['journal_sha256'] = hashlib.sha256(args.journal.read_bytes()).hexdigest()
    if args.annotations:
        result['annotation_sha256'] = hashlib.sha256(args.annotations.read_bytes()).hexdigest()
    require(args.output.resolve() not in {args.journal.resolve(),
            args.annotations.resolve() if args.annotations else args.journal.resolve()},
            'Output must not overwrite source evidence')
    body = json.dumps(result, indent=2, allow_nan=False)+'\n'
    if args.output.exists():
        require(args.output.read_text() == body, 'Output already contains different results')
    else:
        args.output.write_text(body)
        args.output.chmod(0o600)
    print(json.dumps({k: v for k, v in result.items() if k != 'appearance'}))


if __name__ == '__main__':
    main()

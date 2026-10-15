#!/usr/bin/env python3
"""Select on development rows only; report held-out rows separately after selection."""
import argparse
import json
from pathlib import Path


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('report', type=Path)
    args = p.parse_args()
    report = json.loads(args.report.read_text())
    development = set(report['seed_split']['development'])
    held_out = set(report['seed_split']['held_out'])
    if not development or not held_out or development & held_out:
        p.error('Expected disjoint nonempty development/held-out seeds')
    # Freeze these relative budgets before consulting held-out rows. This ranks
    # tradeoffs, including failure to reach a proposed gate; it never declares
    # live usability or changes a receiving config.
    def eligible(row, seeds):
        return row['seed'] in seeds and row['fps'] in (10, 20) and row['noise_amplitude'] == 0.009
    def score(rows):
        return max(max(r['stationary_rms_px']/5,
                       r['moving_rms_px']/(10 if r['trajectory'] == 'fine' else 20)) for r in rows)
    profiles = sorted({r['profile'] for r in report['rows']})
    candidates = []
    for name in profiles:
        rows = [r for r in report['rows'] if r['profile'] == name and eligible(r, development)]
        candidates.append((score(rows), name))
    objective, selected = min(candidates)
    validation = [r for r in report['rows'] if r['profile'] == selected and eligible(r, held_out)]
    print(json.dumps(dict(schema='wigglecontrol.simulation-selection/v1',
        method='Minimize worst normalized rest/fast/fine RMS at10/20Hz high noise using development seeds only',
        budgets=dict(rest_rms_px=5, fast_rms_px=20, fine_rms_px=10),
        selected=selected, development_worst_budget_ratio=objective,
        held_out_worst_budget_ratio=score(validation),
        held_out_rows=validation, live_config_changed=False,
        qualification='Analytic simulation candidate only; fixed budgets may still fail'), indent=2))


if __name__ == '__main__':
    main()

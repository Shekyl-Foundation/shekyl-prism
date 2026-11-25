# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause
"""Builds the tables of REPORT.md from bounds.py and results/raw/*.json.

Writes results/empirical_vs_proven.csv, results/summary.json and
results/tables.md.
"""
import csv
import json
import os

from q5common import SETS, params
from bounds import start_bounds, intermediate_bounds

HERE = os.path.dirname(os.path.abspath(__file__))
RES = os.path.join(HERE, 'results')

CLASSES = [  # (label, intermediates)
    ('starting norms N4 (in)', ['init.nrd.N4']),
    ('coordinates, step and conversion', ['in.coord', 'state.coord', 'upd.v', 'cl.upd.v', 'conv.coord']),
    ('quotient x coordinate (m u)', ['upd.mu_u', 'cl.upd.mu_u']),
    ('update temporaries a = mu u', ['upd.a', 'cl.upd.a']),
    ('Hermitian products / partial sums / h', ['herm.prod', 'herm.partial', 'herm.h', 'cl.herm.prod', 'cl.herm.partial', 'cl.herm.h']),
    ('j-part sums before x p', ['herm.partial_j', 'nrd.partial_j', 'cl.herm.partial_j', 'cl.nrd.partial_j']),
    ('norms N4 and partial sums', ['nrd.prod', 'nrd.partial', 'nrd.N4', 'cl.nrd.prod', 'cl.nrd.partial', 'cl.nrd.N4']),
    ('64-bit quotient |m|', ['q64.m']),
    ('64-bit quotient internals (hs, num)', ['q64.hs', 'q64.num']),
    ('cleanup exact quotient |m|', ['cl.m']),
    ('cleanup rounding numerator 2h + nu', ['cl.round_num']),
    ('cleanup rounding denominator 2 nu', ['cl.round_den']),
    ('4-D Gram entries (N4 scale)', ['conv.gram']),
    ('4-D Gram entries (reference 2x scale)', ['conv.gram_ref2x']),
]


def proven_for(P, rep, variant):
    for v, M0, S, note in start_bounds(P, rep):
        if v == variant:
            Ms, b = intermediate_bounds(P, M0, S)
            return M0, Ms, b, S
    raise KeyError(variant)


def max_B(P, rep, variant):
    """Largest Q4 bound B for which the variant's largest intermediate still
    fits the prism-int width (everything else fixed)."""
    lo = P['B']
    for B in range(P['B'], P['B'] + 4000):
        Q = dict(P); Q['B'] = B; Q['Tbits'] = B + 2 * P['e'] - 5
        _, _, b, _ = proven_for(Q, rep, variant)
        if max(b.values()).bit_length() > P['width']:
            return B - 1
        lo = B
    return lo


def main():
    rows = []
    summary = {}
    md = []
    for s in SETS:
        name = s[0]; P = params(name)
        raw = os.path.join(RES, 'raw', name + '.json')
        if not os.path.exists(raw):
            print('missing', raw); continue
        d = json.load(open(raw))
        per_var = {}
        for f in d['families']:
            M0, Ms, b, S = proven_for(P, f['rep'], f['variant'])
            for k, ob in f['maxbits'].items():
                pb = b[k].bit_length() if k in b else None
                rows.append(dict(set=name, family=f['name'], rep=f['rep'], variant=f['variant'], n=f['n'],
                                 intermediate=k, observed_bits=ob, proven_bits=pb,
                                 slack_bits=(pb - ob) if pb is not None else None))
            key = (f['rep'], f['variant'])
            pv = per_var.setdefault(key, dict(n=0, obs_max=0, obs_max_noinit=0, rel_viol=0, conv_checked=0,
                                              conv_fail=0, nsat=0, sat_runs_ideal=0, growth=0.0, fsr=0, fsr_mm=0,
                                              families=[], obs={}))
            pv['n'] += f['n']; pv['families'].append(f['name'])
            pv['obs_max'] = max(pv['obs_max'], max(f['maxbits'].values()))
            pv['obs_max_noinit'] = max(pv['obs_max_noinit'], max(v for k, v in f['maxbits'].items()
                                                                 if not k.startswith('init.') and k != 'in.coord'))
            pv['rel_viol'] += f['n_rel_viol']; pv['conv_checked'] += f['conv_checked']
            pv['conv_fail'] += f['conv_fail']; pv['nsat'] += f['nsat']
            pv['growth'] = max(pv['growth'], f['max_log2_growth'])
            pv['fsr'] += f['fsr_checked']; pv['fsr_mm'] += f['fsr_mismatch']
            for k, ob in f['maxbits'].items():
                pv['obs'][k] = max(pv['obs'].get(k, 0), ob)
        summary[name] = dict(guard_stats=d['guard_stats'], guard_rate=d['guard_rate'], seconds=d['seconds'],
                             variants={})
        for (rep, var), pv in per_var.items():
            M0, Ms, b, S = proven_for(P, rep, var)
            top = max(b.values()).bit_length()
            summary[name]['variants'][f'{rep}:{var}'] = dict(
                n=pv['n'], families=pv['families'], proven_max_bits=top, M0_bits=M0.bit_length(),
                observed_max_bits=pv['obs_max'], observed_max_bits_excl_input=pv['obs_max_noinit'],
                width=P['width'], spare_proven=P['width'] - top, spare_observed=P['width'] - pv['obs_max'],
                lemma_violations=pv['rel_viol'], oracle_checked=pv['conv_checked'], oracle_fail=pv['conv_fail'],
                saturated_quotients=pv['nsat'], max_log2_Mmax_over_M0=pv['growth'],
                fsr_checked=pv['fsr'], fsr_mismatch=pv['fsr_mm'],
                classes={lab: dict(proven=max(b[k].bit_length() for k in ks if k in b),
                                   observed=max([pv['obs'].get(k, 0) for k in ks]))
                         for lab, ks in CLASSES})
        summary[name]['max_B_fit'] = {f'{rep}:{v}': max_B(P, rep, v) for rep, v in
                                      (('O0', 'sign_generic'), ('O0', 'sign_any_shape'))}
        summary[name]['B'] = P['B']
    with open(os.path.join(RES, 'empirical_vs_proven.csv'), 'w', newline='') as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0].keys())); w.writeheader(); w.writerows(rows)
    json.dump(summary, open(os.path.join(RES, 'summary.json'), 'w'), indent=1)

    # tables.md
    md.append('## Headline: largest intermediate, proven vs observed (bits), against the prism-int width\n')
    md.append('| Set | Variant (rep) | runs | proven max | observed max | width | spare (proven) | spare (observed) | lemma viol. | oracle fail | saturated q |')
    md.append('|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|')
    for name, sm in summary.items():
        for key, v in sorted(sm['variants'].items()):
            md.append(f"| {name} | {key} | {v['n']} | {v['proven_max_bits']} | {v['observed_max_bits']} | "
                      f"{v['width']} | {v['spare_proven']} | {v['spare_observed']} | {v['lemma_violations']} | "
                      f"{v['oracle_fail']}/{v['oracle_checked']} | {v['saturated_quotients']} |")
    md.append('\n## Per intermediate class, signing generic path in O0 numerators (proven / observed bits)\n')
    labs = [lab for lab, _ in CLASSES]
    md.append('| Intermediate | ' + ' | '.join(summary.keys()) + ' |')
    md.append('|---|' + '---:|' * len(summary))
    for lab in labs:
        cells = []
        for name, sm in summary.items():
            v = sm['variants'].get('O0:sign_generic')
            c = v['classes'][lab] if v else None
            cells.append(f"{c['proven']} / {c['observed']}" if c else '-')
        md.append(f'| {lab} | ' + ' | '.join(cells) + ' |')
    md.append('\n## Keygen guard nrd(v_s) <= c sqrt(p) N on sampled keys: rejections\n')
    md.append('| Set | keys | c=2 | c=4 | c=16 | c=256 | c=2^16 | max nrd(v_s)/(sqrt(p) N) |')
    md.append('|---|---:|---:|---:|---:|---:|---:|---:|')
    for name, sm in summary.items():
        g = sm['guard_rate']; r = g['rejected']
        md.append(f"| {name} | {g['n']} | {r['2']} | {r['4']} | {r['16']} | {r['256']} | {r['65536']} | "
                  f"{g['max_nrd_v_over_sqrtp_N']:.2f} |")
    md.append('\n## Q4 slack: largest B for which the reduction still fits the width\n')
    md.append('| Set | B (Q4) | max B, generic shape | max B, any shape |')
    md.append('|---|---:|---:|---:|')
    for name, sm in summary.items():
        mb = sm['max_B_fit']
        md.append(f"| {name} | {sm['B']} | {mb['O0:sign_generic']} | {mb['O0:sign_any_shape']} |")
    open(os.path.join(RES, 'tables.md'), 'w').write('\n'.join(md) + '\n')
    print('\n'.join(md))


if __name__ == '__main__':
    main()

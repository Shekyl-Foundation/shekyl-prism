# Copyright (c) 2025-2026, The Shekyl Foundation
#
# All rights reserved.
# BSD-3-Clause

"""Fresh-sample cross-check: q divides nrd(beta) and beta is in the lattice
for the exact constructions; SUPER is expected to fail both.
"""
from srl import (SETS, STREAM_NAMES, PROD_NAMES, shake,
                 stream_job_data, run_stream_construction, run_prod_construction)


def main(n=4):
    fails = 0
    checks = 0
    for set_name, e, f in SETS:
        for i in range(n):
            sk = shake(b'cc-sk', set_name.encode(), i.to_bytes(4, 'little'), n=32)
            msg = shake(b'cc-msg', set_name.encode(), i.to_bytes(4, 'little'), n=32)
            data = stream_job_data(sk, msg, set_name, e, f)
            for name in STREAM_NAMES:
                out = run_stream_construction(name, data)
                checks += 1
                if name == 'SUPER':
                    if out['ok_q'] or out['ok_in']:
                        print('UNEXPECTED SUPER success', set_name, out)
                        fails += 1
                elif name == 'Q256':
                    pass  # may or may not stay positive-definite
                elif not (out['ok_q'] and out['ok_in']):
                    print('FAIL', set_name, name, out)
                    fails += 1
            for name in PROD_NAMES:
                out = run_prod_construction(name, sk, data)
                checks += 1
                if not (out['ok_q'] and out['ok_in'] and out['ok_extra']):
                    print('FAIL', set_name, name, out)
                    fails += 1
        print(set_name, 'checked')
    print('checks', checks, 'fails', fails)
    return fails


if __name__ == '__main__':
    raise SystemExit(main())

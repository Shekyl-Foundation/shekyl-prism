import sys, time
from gmpy2 import mpq
import sys, os; sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "vendor")); import certify as C
l = int(sys.argv[1]); tgt = int(sys.argv[2])
t = time.time(); ok, n, f = C.certify_tail_exact(mpq(1, 1 << l), tgt)
print(l, tgt, ok, n, f if not ok else '', f'{time.time()-t:.1f}s', flush=True)

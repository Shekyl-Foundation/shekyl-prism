"""Shared helpers for the follow-up (t49u).  Imports the earlier oracle (oracle.py, gen_corpus.py in the
parent directory, merged at d5acb8e); nothing from the prototype."""
import os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
from oracle import *            # noqa
from gen_corpus import *        # noqa
from oracle import _pari

def col_hnf(vectors):
    """Canonical *column* Hermite normal form of the lattice spanned by `vectors` (raw O0 numerator
    coordinates), by PARI mathnf.  Convention (documented in FOLLOWUP.md): M is 4x4, the columns of M are a
    Z-basis, M is upper triangular (M[i][j] = 0 for i > j), M[i][i] > 0, and 0 <= M[i][j] < M[i][i] for j > i.
    This is the layout of the prototype's GenericHnf columns, e.g. (2N,0,0,0),(0,2N,0,0),(x,y,1,0),(2N-y,x,0,1).
    Returned as a list of 4 rows of 4 integers, M[i][j], row-major."""
    n = len(vectors)
    flat = [vectors[j][i] for i in range(4) for j in range(n)]
    H = _pari.mathnf(_pari.matrix(4, n, flat))
    M = [[int(H[i, j]) for j in range(4)] for i in range(4)]
    assert all(M[i][j] == 0 for i in range(4) for j in range(i))
    assert all(M[i][i] > 0 for i in range(4))
    assert all(0 <= M[i][j] < M[i][i] for i in range(4) for j in range(i + 1, 4))
    return M

def lattice_of_col_hnf(M):
    return hnf_rows([tuple(M[i][j] for i in range(4)) for j in range(4)])

def hxs(v):
    return ("-" if v < 0 else "") + format(abs(v), "x")

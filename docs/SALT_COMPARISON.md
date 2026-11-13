# The challenge with and without a salt

Status: RESULTS, 2026-11-13. Measurements for PR #15, taken at commit
`9d2aef5bb6eb8d00402c591fd376b9390b37b88e` unless a record says otherwise;
every record's `.provenance` names its commit.
Claims are marked **Ran** (executed, raw output committed) or **Derived**
(arithmetic or argument on what ran). The protocol is in
[SALT_PROTOCOL.md](SALT_PROTOCOL.md) and the argument in
[SECURITY_ARGUMENT.md](SECURITY_ARGUMENT.md). This is a research
comparison, not a security level for any row.

## The decision

The signature now carries a salt of 32, 48 or 64 bytes in place of the
two-byte challenge index, and the challenge is
`cSHAKE256("shekyl-prism-challenge-v2"; pk || salt || message)`. Without
the salt, anyone could prepare two messages with one challenge, obtain a
signature on the first and present it for the second, at about the square
root of the challenge space; with it, the same forger needs a second
preimage onto a challenge the signer has already signed.

## E2. Cost at the full width

**Derived** from the densities of E1 below. Hash queries for constant
success in the random-oracle estimate, as powers of two. Constant
factors, memory and the cost of one hash are left out, so these are not
AES-equivalent figures.

| Row | Free bits b | Pooled density | Unsalted collision replay | Salted, one signed target | Salted, 2^64 signed targets | Salt bits k | Bound per unit cost beyond Problem 3 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| I | 244 | 1.055% | 2^125.3 | 2^244 | 2^180 | 256 | 2^-180 + 2^-184.4 = 2^-179.93 |
| III | 372 | 0.695% | 2^189.6 | 2^372 | 2^308 | 384 | 2^-308 + 2^-311.8 = 2^-307.90 |
| V | 496 | 0.584% | 2^251.7 | 2^496 | 2^432 | 512 | 2^-432 + 2^-439.6 = 2^-431.99 |

- *Unsalted collision replay* is `sqrt(2^b/rho)`: a birthday search among
  the `rho·2^b` scalars of prime degree, each of which costs `1/rho`
  hashes to hit.
- *Salted, signed targets* is `2^b/n` for `n` signatures: the forged salt
  is free, so the search is over the whole digest space, for any of the
  signed scalars.
- *Bound per unit cost* is equation (18) of ePrint 2026/443 without its
  isogeny term, `N_sign/2^b + N_sign/(rho·2^(k−1))`, at `N_sign = 2^64`.
  It follows from Lemma 1 and Theorem 2 equation (12) with digest space
  `2^b`, accepted fraction `rho` and salt space `2^k`. The brief's derived
  figure of about `2^-180` at row I holds. The mapping, and the
  obligations it leaves (pseudorandom salts, the distribution of failed
  candidates, the isogeny problem actually assumed, EUF against SUF), are
  in the security argument §6.

### E1. Density of prime degrees

**Ran.** `10^5` challenges per row and hash version at the verifier's
width, with the production projection and Baillie–PSW test. Record:
[e1/density.txt](comparison/e1/density.txt).

| Row | Hash | Samples | Prime degrees | Density | 95% interval |
| --- | --- | ---: | ---: | ---: | --- |
| I | challenge-v1-unsalted | 100000 | 1054 | 1.054% | 0.993% to 1.119% |
| I | challenge-v2-salted | 100000 | 1056 | 1.056% | 0.994% to 1.121% |
| III | challenge-v1-unsalted | 100000 | 682 | 0.682% | 0.633% to 0.735% |
| III | challenge-v2-salted | 100000 | 708 | 0.708% | 0.658% to 0.762% |
| V | challenge-v1-unsalted | 100000 | 579 | 0.579% | 0.534% to 0.628% |
| V | challenge-v2-salted | 100000 | 588 | 0.588% | 0.542% to 0.637% |

The two hash versions agree at every row, as they must if both behave as
random functions: the z statistics of the three differences are
−0.04 at row I, −0.70 at row III, −0.26 at row V. The earlier unsalted
measurement (1,500 first-prime searches per row over synthetic keys, security argument A.2) gave 1.089%, 0.719% and
0.526%. Rows I and III agree with it within sampling error. Row V is
higher here, 0.584% against 0.526%, a difference of about 2.6 standard
errors of the two estimates; the E2 exponent moves by less than 0.1 bit
either way.

The density is of degrees passing Baillie–PSW. Treating them as primes
is obligation A2, and treating them as signable is what E6 measures.

## E3. The forgery, carried out at reduced width

**Ran.** Row I, the real native signer and the real algebraic verifier.
The only change is the width `w` of the challenge projection, through the
off-by-default `research-reduced-width` feature; field size, degree
arithmetic, primality test, pairing, both chains and all image checks are
the production ones. A width `w` has `w−1` free bits because the low bit
is forced. Twenty trials at each width, each with its own messages; one
key (the public test seed, `0x42` repeated). Records:
[e3/trial-00.txt](comparison/e3/trial-00.txt) to `trial-19.txt`, one JSON
line per width with the messages, signatures, salts and counts;
per-trial figures in [e3/plot_data.tsv](comparison/e3/plot_data.tsv).

Each trial does three things.

1. **Unsalted replay.** Search two messages whose archived challenge at
   index 0 is the same scalar of prime degree. The archived signing loop
   signs the first. The same bytes are submitted for the second message
   to the archived derivation in front of the real verifier.
2. **The same attacker against the salt.** Search two messages colliding
   under a salt the attacker guessed (all zeros), under the salted hash
   so that only the unknown salt differs. The salted signer signs the
   first. The signature is submitted for the second.
3. **Best attack against the salt.** Take that signature and search a
   message hashing, under its salt, onto its scalar. Submit it.

| Width w | Free bits | Unsalted replays accepted | Unsalted hash queries, geometric mean (min to max) | Precomputed collisions accepted, salted | Second-preimage hash queries, geometric mean (min to max) | Second preimages accepted | Default verifier accepted |
| ---: | ---: | ---: | --- | ---: | --- | ---: | ---: |
| 16 | 15 | 20/20 | 2214 (886 to 4761) | 0/20 | 37420 (2638 to 149951) | 20/20 | 0/20 |
| 20 | 19 | 20/20 | 6682 (1393 to 12941) | 0/20 | 323384 (21655 to 1782901) | 20/20 | 0/20 |
| 24 | 23 | 20/20 | 29953 (3656 to 76347) | 0/20 | 7004550 (1966548 to 28094923) | 20/20 | 0/20 |
| 28 | 27 | 20/20 | 107321 (40648 to 253550) | 0/20 | 75144653 (4748967 to 377462408) | 20/20 | 0/20 |
| 32 | 31 | 20/20 | 484381 (137022 to 1399211) | 0/20 | 1522476477 (42372576 to 13540406681) | 20/20 | 0/20 |

Least-squares fit of `log2(queries)` on `w`, 100 points each (five widths, twenty trials):

| Attack | Slope | Standard error | Intercept | Predicted slope |
| --- | ---: | ---: | ---: | ---: |
| Unsalted collision replay | 0.489 | 0.015 | 3.13 | 0.5 |
| Salted second preimage | 0.962 | 0.030 | -0.51 | 1.0 |

All 100 unsalted replays and all 100 second preimages were accepted by
the reduced-width verifier; all 100 precomputed collisions were refused
under the signer's salt. In the 100 salted signing calls no backend
error, malformed result or refused response advanced the counter, and
every unsalted collision was signed at the first attempt.

**Derived** before the run: slopes near 0.5 and 1.0. The fitted values
are as predicted. The unsalted replay costs the square root of the
challenge space and the salted second preimage costs the space itself,
against one signed target.

No reduced-width forgery is accepted by the default verifier, which
derives the full-width challenge. The full-width costs in E2 are an
extrapolation of these slopes over 212 further bits, on the assumption
that cSHAKE256 has no structure an attacker can use. Nothing at the full
width was attempted.

The salted figure is for one signed target. With `n` signatures the same
search succeeds `n` times sooner, which is the `2^b/n` column of E2.

## E5. What the index showed

**Ran.** For each of the 113 archived unsalted signatures, the number of
earlier indices whose degree is prime, from the public key, message and
carried index alone. Record:
[e5/index_leak.txt](comparison/e5/index_leak.txt).

| Earlier prime indices | Vectors |
| ---: | ---: |
| 0 | 113 of 113 |

The carried index itself ran from 3 to 472 with median 111.

Every archived signature sits at the first prime index, so on this
corpus the index disclosed nothing beyond what the message already
determines. The disclosure is real but was not exercised: the archived
signer moved past a prime index only when its own verifier refused the
response, and E6 counts how often that happened on fresh messages.

Under the salt the count cannot be computed. The signature carries
`salt_i` and not the counter `i`, and the salts of the earlier counters
are outputs of a function keyed by the signer's secret `salt_key`.
Signing time still depends on the number of candidates.

## E6. The unsignable message

**Ran.** Both signing loops on the same messages at the full width, on
the same backend, key and scalar-keyed response stream. Message `n` is
`shekyl-prism-comparison-e6-v1 || LE64(n)`. The search budget was fixed
before the run: 20,000 messages at row I, 4,000 at III, 2,000 at V.
Records: `comparison/e6/level{1,3,5}-*.txt`, one `SIGN` line per message.

| Row | Messages | Unsalted: signed at the first prime index | Unsalted: backend error, exhausted or malformed | Salted: signed at the first prime candidate | Salted: backend errors | Salted: refused responses |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| I | 20000 | 20000 | 0 | 20000 | 0 | 0 |
| III | 4000 | 4000 | 0 | 4000 | 0 | 0 |
| V | 2000 | 2000 | 0 | 2000 | 0 | 0 |

No unsignable message was found in 26,000 tried. Every message was
signed by both loops at its first prime candidate: the archived loop
never skipped a prime index, and the salted loop never advanced its
counter on a backend error, malformed result or refused response. A
backend failure rate above about 1 in 6,700 at row I, 1 in 1,300 at III
or 1 in 670 at V would have shown at least once with 95% probability.
Rarer failures are not excluded, and one key was used.

## E4. Sizes and time

| | I | III | V |
| --- | ---: | ---: | ---: |
| Signature bytes, unsalted | 322 | 482 | 642 |
| Signature bytes, salted | 352 | 528 | 704 |
| Growth (`SALT_LEN − 2`) | 30 | 46 | 62 |

**Ran.** Successful verification on x86_64, `prism-bench` (31 samples
after 5 warmups per row), the archive tag and this branch run alternately
three times, on a development machine that other work was loading. Taken
twice: [e4/verify_time.txt](comparison/e4/verify_time.txt) at 1-minute
load 8 to 13, and
[e4/verify_time_quiet.txt](comparison/e4/verify_time_quiet.txt) the next
day at load 3 to 5, with the salted side at `ef2fba6` (the verifier is
the same code).

Median of the three run medians, in milliseconds:

| Record | Protocol | I | III | V |
| --- | --- | ---: | ---: | ---: |
| First (load 8 to 13) | unsalted | 23.4 | 67.9 | 160.6 |
| First (load 8 to 13) | salted | 26.6 | 73.4 | 153.3 |
| Second (load 3 to 5) | unsalted | 24.1 | 68.9 | 163.4 |
| Second (load 3 to 5) | salted | 23.6 | 68.0 | 153.6 |
| Fastest run of the six, either record | unsalted | 23.1 | 67.1 | 150.0 |
| Fastest run of the six, either record | salted | 23.1 | 66.9 | 150.0 |

Individual run medians range up to twice these figures where another
build was running; the records keep every run.

**Derived.** Verification time is unchanged within the noise of this
machine. The salted verifier hashes 30 to 62 more bytes and otherwise
runs the same relation. These are not idle-machine or floor-device
figures; [BENCHMARKING.md](BENCHMARKING.md) has the Pi 4 measurement of
the unsalted verifier.

Signer candidates per signature, from the E6 run:

| Row | Unsalted, mean hashes | Salted, mean hashes | Salted, mean prime candidates |
| --- | ---: | ---: | ---: |
| I | 91.4 | 92.0 | 1.0000 |
| III | 144.6 | 141.0 | 1.0000 |
| V | 187.4 | 179.7 | 1.0000 |

Both loops hash until the first prime degree, about `1/rho` candidates,
and call the backend once.

## What the salt does not change

- **A signer can still make one signature valid for two messages of its
  own choosing**, at the unsalted collision cost. Whoever chooses the
  salt can collide two messages under it. This is the paper's Remark 13:
  salt-PRISM has `a/2`-bit security against message-bound-signature
  attacks. Step 2 of E3 is this attack with the roles reversed: it fails
  only because the attacker there does not choose the salt.
- **A predictable salt returns the scheme to the unsalted cost.** The
  salt protects because it is unknown before signing. If the salt key
  leaks, or salts are derived from public data, or an attacker can make
  the signer use a salt it chose, step 1 of E3 applies unchanged. Zero
  caller randomness is not this case: the salt is still a secret-keyed
  function of the message.
- **The polarized-map, primality and encoding obligations (A1 to A3) are
  untouched.** They concern the acceptance relation, not the challenge
  hash. The isogeny assumption and the signer's side channels are
  likewise as open as before.

## Records and reproduction

Every record under `docs/comparison/` has a `.provenance` file with the
commit, toolchain, driver and backend digests, command, start and end
time, load and exit status. Build the native backend as in the README,
then:

```sh
sh scripts/run_comparison.sh e1/density density 100000
sh scripts/run_comparison.sh e5/index_leak index_leak
sh scripts/run_comparison.sh e3/trial-00 replay target/native 0 all     # trials 0 to 19
sh scripts/run_comparison.sh e6/level1-0 unsignable target/native 1 0 5000
python3 scripts/analyze_comparison.py
```

The runner refuses a tree whose tracked files differ from `HEAD` and
never overwrites a record. `analyze_comparison.py` recomputes every table
above from the records. The E3 and E6 runs used up to 28 concurrent
processes at reduced priority; their query counts do not depend on load,
and the seconds in the E3 records do. CI runs E3 at widths 16 and 20 on
every pull request (`scripts/ci/check_replay_regression.py`).

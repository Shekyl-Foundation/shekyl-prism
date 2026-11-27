# G7 timing: RTLF next to Welch t, matched preamble (PARTIAL, in progress)

Status: partial commit to keep progress. Tables and the gate rule follow in the final version.

Done so far
- RTLF reference pinned: tls-attacker/RTLF 2497d660e4b23fc9acd21b077a35e82178fcd728 (see `pin.txt`).
- Matched-preamble harness: `harness-matched.patch` (on dev 470d537, cumulative with the p248 split patch).
- Pi campaign (node not running, see below): runG and runH raw data in `results/data/`.
- Positive control on recorded samples and on the Pi (`posctl.py`, `pi/campaign.sh`).
- Bulk RTLF runs use `rtlf_np.py`, a port validated against the R reference. The R reference is still running over all 2711 re-analysed contrasts.

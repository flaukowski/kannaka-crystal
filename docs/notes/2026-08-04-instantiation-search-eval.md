# Does searching instantiation produce a capability crossing?

2026-08-04. behavior-contract-v2 (v0.15.0), metamaterial, `pattern_completion`
and `noise_shielding`.

## The question

v1 held placement, scale, amplitude and orientation fixed. Directed
evolution under it enriched all 86 candidates it touched and crossed
nothing, and more generations never moved the ceiling — the signature of
a bound outside the thing being optimized. So: make instantiation
searchable and see whether the ceiling was ever really about which
structures exist.

## Protocol

Search placement by coordinate descent on a set of fit seeds, then
measure the found placement on disjoint held-out seeds. Run the
identical search on a deterministic permutation of the signature as a
control. Pass requires held-out mean ≥ 0.01, ≥70% positive trials, and
held-out > control.

## Result 1 — in-sample looks wonderful and does not survive

Ten runs (5 replicated primitives × 2 capabilities), 4 fit seeds, 30
held-out:

    mean in-sample   +0.0277
    mean held-out    +0.0016
    collapse         17x

Every one of the ten cleared the bar in-sample by two to four times.
None cleared it held-out. A placement chosen on four seeds does not help
on the next thirty.

## Result 2 — the control was manufacturing advantage too, until it wasn't

Sweeping fit-seed count on two primitives (`pattern_completion`, 30
held-out):

| fit seeds | CRY-012818 in→held (control) | CRY-012850 in→held (control) |
|-----------|------------------------------|------------------------------|
| 4         | +0.0309 → +0.0012 (+0.0057)  | +0.0348 → +0.0099 (+0.0116)  |
| 12        | +0.0245 → +0.0006 (+0.0039)  | +0.0275 → +0.0103 (+0.0104)  |
| 24        | +0.0195 → **+0.0098** (−0.0037) | +0.0263 → +0.0003 (−0.0038) |

Two things move together as fit seeds grow. In-sample inflation shrinks
(0.0309 → 0.0195), which is ordinary. More interesting: **the scrambled
control collapses from positive to negative** (+0.0057 → −0.0037;
+0.0116 → −0.0038). At four fit seeds a scramble can find a placement
that looks good; at twenty-four it cannot.

That is the control working. It also means results computed at small
fit-seed counts are uninterpretable in both directions — the earlier
CRY-012630 "pass" at 6 held-out seeds sat in exactly that regime.

## Result 3 — the near miss was noise

CRY-012818 at 24 fit seeds looked like the closest thing to a crossing:
held-out **+0.0098** against a control of **−0.0037**, separating from
its own scramble by more than the width of the bar, and failing only
because 0.0098 is not 0.01.

Re-measured at 60 held-out seeds it reads **+0.0015 ± 0.0568, 48%
positive**. Standard error is 0.0073, so the estimate is indistinguishable
from zero and the +0.0098 was a 30-seed fluctuation.

Two lessons, and the second is the one that cost the most time. First:
never argue from a number whose error bar you have not looked at — the
spread here is nearly six times the bar being tested. Second: this is the
third time in one session that a promising result has evaporated under
more seeds (CRY-012630 at 6→30, CRY-012818 at 30→60). Every apparent
crossing so far has been small-sample noise, and the only reason any of
them was caught is that the contract records the control and the
in-sample gap alongside the claim.

Had the bar been relaxed to "separates from control" when +0.0098 looked
tantalizing, this primitive would now be Level 6 and wrong.

## Reading

"The ceiling is the fixed instantiation" is not supported as stated.
Searching placement moves the in-sample number a great deal and the
out-of-sample number very little. Whatever the ladder's sixth rung is
waiting on, letting structures be placed better does not by itself
deliver it.

Stated as a measurement rather than a mood: across 13 held-out
evaluations spanning three fit-seed counts and three held-out counts,
mean out-of-sample advantage is ≈ +0.002 against a 0.01 bar, and no
configuration produced a value that survived re-measurement.

The weaker claim survives: v1's fixed placement is measurably a bad one.
It scored negative for `noise_shielding` on all five fresh candidates
(−0.0035 to −0.0081) — dead centre at gain 3.0 is not neutral, it is
actively unhelpful for that task. Removing a bad constant is worth doing;
it just did not turn out to be the bound.

## Defect found while chasing this

A failed contract did not withdraw the Level 6 a passing one granted.
CRY-012630 sat at L6 in the live registry with both capability records
reading `passed: false` — a level held on the strength of a run no longer
in the record. Level 6 is now recomputed from current records and falls
back to whatever non-behavioral evidence still supports; that primitive
now reports L4. `reproduce` has always demoted on failure. Behavior
should never have been the rung that only ratchets upward.

## Next

Contract-side, not search-side. Both v1 and v2 instantiate a structure
once, at the start, into a field that then evolves without it — the
structure is furniture, not a participant. A contract where the structure
is re-asserted during evolution, or coupled to the task rather than
merely present in it, tests a different claim than either version has
tested so far.

---

# Addendum, same day: behavior-contract-v3 — presence instead of placement

The contract-side follow-up proposed above was built and measured within
the day. v3 keeps the structure present — re-asserted every N steps at a
fraction of its gain while the task runs — instead of painted once into
a field that then evolves without it. Selection is over nine
(interval, strength) combinations at the default placement; same
fit/held-out split; the scramble receives the identical sustained
injection.

## The first number all day that did not evaporate

CRY-012705, presence every 25 steps at 1.0× gain:

| capability | 30 held-out | 120 held-out | control @120 | positive @120 |
|---|---|---|---|---|
| noise_shielding | +0.0142 ± 0.0414 | **+0.0125 ± 0.0424** | +0.0090 | 60% |
| pattern_completion | +0.0113 ± 0.0462 | **+0.0096 ± 0.0484** | +0.0039 | 60% |

Every v2 result collapsed toward zero under a seed increase
(+0.0159→+0.0016; +0.0098→+0.0015). These held: at 120 seeds the
noise-shielding mean is 3.2 standard errors from zero. Sustained
presence produces a real, replicable positive effect. The mechanism
direction — participant, not furniture — is supported.

## Why it still fails, and should

Both runs record FAILED, correctly, for two reasons that are findings in
their own right:

1. **60% positive < the 70% floor.** Presence helps on most seeds and
   actively hurts on a large minority. The effect is real on average and
   unreliable per-instance — which for a capability claim is a fail; a
   shield that fails four seeds in ten is not yet a shield.

2. **On noise_shielding, most of the effect is wattage.** The scrambled
   control reads +0.0090 and +0.0094 across independent runs — sustained
   injection of ANY pattern nearly clears the bar on that task. The
   arrangement's own contribution is ~+0.0035. Pattern completion is
   more encouraging: its control sits at +0.0039–0.0042, so arrangement
   carries ~60% of a near-bar effect there.

The demotion rule shipped this morning fired twice, live: the 30-seed
pass granted L6, and the 120-seed re-test took it back. Final state
L4 — exactly what the primitive's non-behavioral evidence supports.

## Where this leaves the ladder

Not a crossing. The claim "presence confers capability" survives at the
population level and fails at the instance level, and the next lever is
visible in the two controls: pattern_completion is the task where
arrangement does the work, so per-seed reliability there — why those
40% of seeds get hurt — is the specific question. Candidates: presence
interval interacting with the task's write timing, or re-assertion
fighting the field's own settled modes on some trajectories (the same
mixing that erases abandoned structures may be what presence disrupts
when it hurts).

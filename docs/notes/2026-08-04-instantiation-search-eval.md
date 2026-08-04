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

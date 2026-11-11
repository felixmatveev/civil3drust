# Profiles and profile views — first look (S298–S305)

Samples from the owner (Civil 3D 2024), one change per step, sent
without dialog values:

| ID | step |
|---|---|
| S298 | base: one alignment `prpfile` (8C4A: tangent, free curve R 430.15, tangent; 1742.48) |
| S299 | profile view (8D70) and layout profile `test` (8D72), two tangents |
| S300 | vertical curve at the middle PVI |
| S301 | PVI edited (its elevation 10 → 0) |
| S302 | alignment edited (curve R 430.15 → 605.96; first tangent 688.27 → 639.04) |
| S303 | second layout profile `test2` (8D91), two tangents, same view |
| S304 | second profile view (8DA9) |
| S305 | first view (8D70) deleted |
| S306 | S305 with the vertical curve's length set to 150 |

## Objects

A profile view and a profile came in the same step (S299). With each
profile come seven small objects (11 bytes each), hard-owned by it:
`AECC_DB_FLOAT_TANGENT_GRADELEN`, `AECC_DB_FIXED_PARABOLA_HLPT`,
`AECC_DB_FIXED_PARABOLA_HLPTPATSPEAK`, `AECC_ENTITYCIRCLEINFOS`,
`AECC_ENTITYEXTENDPARAM`, `AECC_DB_FIXED_ARC_HLPT`,
`AECC_NOTIFICATIONINFOS` (a second set with the second profile, S303).
Their names suggest layout helpers for the entity types (float tangent
by grade and length, fixed parabola, fixed arc); they hold no data of
their own in these samples.

Per profile there are four label groups (`AECC_VALIGNMENT_LINE_`,
`_PVI_`, `_SAGCURVE_`, `_CRESTCURVE_LABEL_GROUP`); deleting the view
deletes them (S305: all eight gone, the profiles stay). Each view has
one `AECC_PROFILEDATABAND_LABELGROUP`.

## Links (handle streams, S304)

- **Alignment → profiles and views:** soft pointers after the alignment
  style: each profile (8D72, 8D91), then each view (8D70, 8DA9). A
  deleted view's pointer goes (S305). These are the "soft pointers to a
  profile and a profile view" seen in a production drawing's offset
  alignment.
- **Profile → alignment:** hard pointer after the profile style (228E);
  before it the layer, the seven helpers (hard owner), the feature style
  twice.
- **View → alignment, profiles:** hard pointer to the view style, the
  alignment, then per profile shown `{soft pointer to the profile, hard
  pointer to its style (null in the second view), null}`, the data band
  style, the alignment again, three more view styles.

`aecc_profile::pointers_to` and `parent_alignment` read them.

## Profile record (`AECC_PROFILE`, class `AeccImpVAlignment`)

Strings: `AeccImpVAlignment`, two empty, then the label point names
(`A`, `HP`, `Overall LP`, `EVC`, `BVC`, `K`, `EVCE`, `Overall HP`,
`BVCE`, `BREAK`, `LP`, `EVCS`, `BVCS`, `PVI`), the name (`test`), empty
strings. Graphics: only a placeholder frame (the profile is drawn by the
view).

The geometry is in **element records shaped like an alignment's**: an
11-bit kind prefix, a header, links by ID, then the data. The header is
23 bits — `00`, `0x08`, `0x00`, five 0 bits — where an alignment's is
26 or 34.

```text
prefix 2, tangent:  BL 0, BL id, BL before, BL after,
                    RD station, RD elevation   (start)
                    RD station, RD elevation   (end)
prefix 6, curve:    BL 0, BL id, BL before, BL after,
                    BD 0, BD length (S300 488.7677; S306 150),
                    BL 0x1001, BL 0, 3 bits, BD −1, BD −1, BD −1, BL 0
```

S299's tangents are (−100, −20) → (871.2385, 10) and (871.2385, 10) →
(1900, −10): the PVIs as drawn, the outer ones at the profile view's
edges (the view spans stations −100 to 1900), not clipped to the
alignment (0 to 1742.48). The vertical curve (S300) joins tangents 1 and
2 (their links become 1 → 3 → 2); its record keeps no points, so it is
placed from the tangents like a free curve. Civil 3D asks only for the
curve length (owner), and that BD is it: S306 (S305 with the length set
to 150) stores exactly 150 there and nothing else in the profile
changes. S300's 488.7677 was the length as first drawn (BVC 626.85, EVC
1115.62 around the PVI).

**PVI edit** (S301): both tangents' shared point changes (elevation 10
→ 0); the curve record does not change.

**Alignment edit** (S302): the alignment's curve radius changed and its
first tangent got 49.2276 shorter; the PVI's station moved back by the
same 49.2276 (871.2385 → 822.0110), the outer points stayed. Civil 3D
keeps the PVI at the same place relative to the alignment geometry
before it; the profile record is rewritten, the view keeps its station
range.

## Checked against the Civil 3D API (S306)

`C3DDUMP` on S306 (owner; `samples/S306_profile_curve_length_150.api.txt`)
agrees with the decoded records to 1e-9 (test
`layout_matches_civil3d_api`, `aecc_profile::layout`):

- PVIs: the tangents' through points — (−100, −20), (822.0110, 0),
  (1900, −10); `test2`: (−100, −32.5), (885.3402, −10), (1900, −30).
  The profile itself runs −100 to 1900 (`StartingStation`,
  `EndingStation`, `Length` 2000): it is **not** clipped to the
  alignment (0–1770.68).
- Tangents: `Constraint1 Fixed`, `Constraint2 PassThroughPt1AndPt2`, the
  through points as stored; their start and end are trimmed by the curve.
- Curve: `ProfileParabolaSymmetric`, `Constraint1 Free`, `Constraint2
  CurveLength`, length 150 at the PVI; BVC 747.0110 / −1.6269, EVC
  897.0110 / −0.6957, K = L / (100 |A|) = 48.4367, high point 852.0785 /
  −0.4873 — all derived from the tangents and the length, none stored.
- `ProfileType FG`, `UpdateMode Static` (a layout profile).
- The `FingerPrint` values (profiles, alignment) are not stored in the
  records as 64-bit values.
- View `prpfile2`: `StationRangeMode Automatic`, stations −100 to 1900
  (the profiles' extent), `ElevationRangeMode Automatic`, elevations
  −32.5 to −0.4873 (the profiles' lowest and highest point); the dialog's
  −60 / 20 are the user-specified fields.

## Profile view record (`AECC_PROFILE_VIEW`)

Its station and elevation ranges appear as BDs (−100, −40, 1900, 30 in
S299–S301: elevation range 70 drawn 700 high, vertical exaggeration
10), with the view's insertion point (1698.18, 165.82); not mapped
field by field yet. The name is `prpfile1` (the alignment's name + 1).

The owner's Profile View Properties for the second view `prpfile2`
(on the current drawing, after S304): station range **Automatic** 0+00.00
to 17+70.68 (the alignment), with the **User specified range** fields
showing −1+00.00 to 19+00.00; elevation range **Automatic height**
−60.00 to 20.00, user fields −60.00, 20.00, height 80.00. So the
station pair −100 / 1900 in the record is the user-specified range,
kept while Automatic is on, and Civil 3D picked it (the owner typed
nothing; both modes are automatic by default with a user override). The
profile's outer points sit on it because the profile was drawn across
the whole view.

## Open

- The vertical curve's first BD (0) and the 3 bits (curve type?
  parabola vs circular, symmetric vs asymmetric — not sampled).
- The data after the element records in the profile.
- The view's fields (ranges, auto/user height, bands).
- The seven helper objects' role.

# Magnetic enclosure shape study

Phase 1 only, not a functional print release. Existing printable stand:
`usb_centered_stand.py` (unchanged so far).

Proposed shell: 48 W × 58 H × 28 D mm, 2.4 mm walls, R6 outline corners,
flat display face, removable rear cover, no stand. Reuses the corrected board
reference and the existing 34.8 × 10.8 × 52 mm battery allowance.
Magnet: user-confirmed Ø12 × 2.7 mm disc. Sides are viewed from the display.

From `housing/`, run:

```sh
.venv/bin/python magnetic_concept.py --usb left --magnet right
```

Exports concept shell/cover STLs, a preview, and a shape report under
`output/magnetic-concept/usb-left_magnet-right/`. Preview screen, USB and
magnet are surface markers, not openings. The shell has a closed front;
cover retention is not yet modeled. CAD solids and watertight STL meshes
are checked, as is battery overlap with the shell, cover and reference board.

| Magnet | Allowed USB positions |
| --- | --- |
| right | left, up |
| left | right, up |

Invalid/same-side configurations are rejected before construction. The PCB
reference rotates with USB selection. This does not yet establish functional
fit for these variants. In particular, top USB needs a connector reach review:
the top wall is farther from the centered board than the side walls.

Next after shape review:

- Actual display seating relief and USB opening; connector access checks.
- Side magnet pocket with local backing boss (2.4 mm wall alone cannot
  contain a 2.7 mm disc), fitting clearance and retention.
- PCB and battery retainers, wire space and removable cover fit.
- Existing stand USB left/right/up parameterization, rotating PCB, connector
  relief and compatible retainers together while keeping the battery upright.
- All-configuration interference and insertion checks, final STL/STEP exports
  in print orientation, and reviewed previews.

Physical magnet grip, printed tolerances, cable overmould and wire routing
remain unverified. Concept meshes are not ready for a functional print.

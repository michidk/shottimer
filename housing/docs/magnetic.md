# RP2040-LCD-1.28 Magnetic Housing

The approved 48 W × 58 H × 28 D mm shell now has a removable press-fit
backplate, using the same retention approach as the existing desktop housing.
This is a feature prototype pending visual review and a physical fit test.
Superseded shape studies are available in Git history.

## Parameters

| Feature | Value |
| --- | --- |
| Magnet | Ø12 × 2.7 mm disc |
| Glue pocket | Ø12.5 × 3.2 mm deep |
| Glue allowance | 0.25 mm radial; 0.5 mm behind a flush magnet |
| Plastic backing behind pocket | 1.2 mm |
| Default positions, viewed from display | USB left; magnet right |
| Body walls / outline corners / exterior chamfer | 2.4 mm / R6 / 2 mm |
| Backplate locating rim | 3.5 mm deep, 1.4 mm wall |
| Rim clearance / friction ribs | 0.15 mm / four ribs with 0.05 mm interference |
| Rim insertion bevel | 0.4 mm |
| USB opening | 12 × 7 mm, R0.8 |
| Battery allowance | 34.8 × 10.8 × 52 mm |

The rim has battery-clearance breaks at its upper and lower ends. The cover
carries a flat battery backing, lower shelf and lip, side rails, and two short rear
PCB stops. The long third L-shaped PCB finger is omitted as requested. A lower fingernail notch releases the cover. The shelf sits in a
local body recess with 1.4 mm nominal floor thickness underneath. Insert the
PCB straight through the rear. Lower the battery straight down from above into the detached cover,
behind the PCB bridges and between the side rails. Insert the loaded
cover from behind. Actual loose leads and the mating battery plug still need
checking during physical assembly.

The magnet is inserted from outside and glued in with its face flush with the
housing; use the 0.5 mm bottom allowance for adhesive rather than pressing it
all the way to the pocket floor. The pocket is not a press-fit retention feature.

## Build and inspect

Run from `housing/`:

```sh
.venv/bin/python build.py magnetic --usb left --magnet right
```

The build checks geometry, component overlaps, sampled insertion paths,
connector-shell access, and watertight meshes before exporting parts and
rendering the review sheet. It writes to `output/magnetic/usb-left_magnet-right/`:

- `body_print.stl` and `cover_print.stl`, oriented with flat exterior faces down.
- `assembly.step`, including component reference geometry.
- `fit-check.json` and `preview.png`.

Select USB `left`, `right`, or `up`, and magnet `left` or `right`. Matching sides
are rejected. For top USB, the board and display move 5 mm upward so the USB
shell keeps the side version's 2.92 mm setback from the outside wall. The
body and backplate must be generated as a matching pair for each configuration.

PLA, 0.2 mm layers, 3 walls, 15% gyroid infill. Keep the exported orientation;
inspect/support the backplate PCB bridges locally. Physical fit depends on
printer calibration, especially the friction ribs. Use a prototype print before
committing the glued magnet.

## Validation limits and remaining scope

The manufacturer STEP includes a pre-existing invalid component solid. Fit
checks replace any such solid with its conservative bounding box; the report
lists those indices. Original reference geometry remains in the STEP assembly.
CAD checks do not certify actual cable-overmould fit, adhesive strength, magnet
holding force, lead routing, material flex, or all intermediate insertion poses.

USB-position parameterization of the existing desktop stand is still pending.
Its current printable release remains `cad/stand.py`; this new model
does not change that release or its backplate.

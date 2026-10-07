# Shot timer housing

Two parametric, screw-free enclosures for the Waveshare **RP2040-LCD-1.28**
and a MakerFocus 3.7 V 2000 mAh LiPo: a tilted desktop stand and a flat magnetic
housing. Both use a removable press-fit rear cover. Compatibility with the
ESP32-S3 board has not been checked.

![Front views of the desktop stand and magnetic housing](../.github/images/housing-overview.png)

## RP2040-LCD-1.28 Magnetic Housing

The separate [magnetic housing](docs/magnetic.md) uses the approved flat
48 × 58 × 28 mm shell, no stand, and a removable press-fit backplate with a
battery cradle. Its Ø12.5 × 3.2 mm glue pocket holds a Ø12 × 2.7 mm magnet.
USB and magnet sides are selected independently, with same-side combinations
rejected. Default: USB left, magnet right, viewed from the display.

```sh
cd housing
.venv/bin/python build.py magnetic --usb left --magnet right
```

This remains a feature prototype; see its fit report and preview before printing.

## RP2040-LCD-1.28 Desktop Stand

The desktop stand is defined in **`cad/stand.py`** and builds into
**`output/usb-centered/`**. Superseded prototypes are available in Git history.

The desktop stand exports these three files locally:

- `body.stl` — front housing with integrated bezel and floor.
- `cover_print.stl` — rear cover, already oriented for printing.
- `assembly.step` — housing, cover, board and battery.

The **12 × 7 mm** USB opening has **R0.8 mm** corners and is centered on the
corrected CAD connector, with approximately **1.42 mm** clearance to its metal
shell at both front and rear edges.

![Housing from six directions](../.github/images/housing-views.png)

## Automated builds and downloads

[Download latest housing artifact](https://nightly.link/michidk/shottimer/workflows/housing.yml/main/housing.zip)
via [nightly.link](https://github.com/oprypin/nightly.link), which resolves the
`housing` artifact from the latest successful `main` workflow run without requiring
GitHub sign-in. The ZIP includes both housing variants, each with two STL files and one STEP assembly.
The folder and filenames include the target PCB and housing configuration:

```text
rp2040-lcd-1.28-desktop-stand/
  rp2040-lcd-1.28-desktop-stand-body.stl
  rp2040-lcd-1.28-desktop-stand-cover.stl
  rp2040-lcd-1.28-desktop-stand-assembly.step
rp2040-lcd-1.28-magnetic-usb-left-magnet-right/
  rp2040-lcd-1.28-magnetic-usb-left-magnet-right-body.stl
  rp2040-lcd-1.28-magnetic-usb-left-magnet-right-cover.stl
  rp2040-lcd-1.28-magnetic-usb-left-magnet-right-assembly.step
```

Use the body and cover from the same folder. The magnetic download uses USB left
and magnet right, viewed from the display; build other configurations locally.
The magnetic housing remains a prototype pending physical fit verification.

[GitHub build history](https://github.com/michidk/shottimer/actions/workflows/housing.yml?query=branch%3Amain+is%3Asuccess) is the fallback:
open a successful run and download `housing` under **Artifacts** (GitHub sign-in
required). Artifacts are retained for 90 days. For a specific commit, select
its workflow run; the stable artifact name is reused across separate runs.

The [Housing CAD workflow](../.github/workflows/housing.yml) builds housing changes
on `main` and pull requests, and supports manual runs. It checks the full assembly,
sampled insertion paths and USB access for both models before uploading the six CAD files.
There is no release-publishing job and the workflow has read-only repository
permissions. The direct latest-download link uses the third-party nightly.link
service; the actual files remain GitHub Actions artifacts.

Generated `output/` files are ignored by Git. CAD source, the original
board reference and README images are versioned. Fit reports and renders are
produced locally during validation but are not included in the download.
Run `render_docs.py` explicitly to refresh documentation images; CI does not
commit generated images.

## Desktop stand dimensions and construction

| Parameter | Current value |
| --- | --- |
| Housing width × depth × height | 48 × 45.52 × 72.37 mm |
| Display tilt, back from vertical | 20° |
| Screen opening diameter | 33.2 mm |
| USB opening | 12 × 7 mm, R0.8 corners |
| USB metal nose setback from outer wall | 2.92 mm |
| PCB + display thickness | 1.6 + 2.3 = 3.9 mm |
| Nominal battery length × width × thickness | 50 × 34 × 10 mm |
| Battery allowance, length × width × thickness | 52 × 34.8 × 10.8 mm |
| Battery lean toward rear | 10° |
| Battery pocket and wedge width | 37.2 mm |
| Rear locating rim depth / wall | 3.5 / 1.4 mm |
| Rim clearance | 0.15 mm, with four local friction ribs |

The bottom is integral and flat. Three lower hooks catch the PCB rim, while
curved locator bases share the display recess radius. The rear cover carries
the tilted battery support and a full-width lower pocket lip. Its inner rim has
two battery-corner clearance breaks and four friction ribs; there are no screws
or visible latch holes. A lower fingernail notch helps remove the cover.

## Print and assembly

PLA, 0.2 mm layers, 3 walls, 15% gyroid infill. Print the housing bottom-down
and `cover_print.stl` with its flat outer face on the bed. Inspect the slicer’s
bridges and overhangs around the PCB hooks and cover arms; add local supports
if your printer cannot bridge them reliably. Three walls help the small retainers.

Insert the PCB from the rear: approach tilted back about 12°, slightly raised,
then lower and pivot it beneath the three lower lips. Slide the battery over
the pocket lip and lower it against the angled support. Route its leads clear
of the rim and USB socket before pressing the rear cover into place.

The current USB correction is CAD-checked, not yet verified with another print.
Press-fit tightness still depends on printer calibration. The manufacturer’s
battery listing permits ±2 mm; this pocket uses the user-selected nominal size
plus clearance, so check the actual pack before inserting it.

## Rebuild and render

Run from this directory with Python 3.12 and [uv](https://docs.astral.sh/uv/getting-started/installation/).
On Linux the renderer also needs an EGL/OpenGL runtime and DejaVu fonts; on
Ubuntu (as in CI), install them with:

```sh
sudo apt-get install -y libegl1 libgl1 libgl1-mesa-dri libglu1-mesa fonts-dejavu-core
```

Without a display, render headlessly as CI does with `PYOPENGL_PLATFORM=egl`
and, without a GPU, `LIBGL_ALWAYS_SOFTWARE=1`.

```sh
cd housing                         # from the repository root
uv sync --locked --python 3.12
.venv/bin/python build.py stand
```

`uv.lock` pins the full dependency graph; its PyOpenGL override supports the
NumPy runtime used here. The command
builds the current assembly, checks component and insertion clearances plus USB
access, exports STL/STEP files, and renders the six-view review sheet.
To refresh the README images after rebuilding:

```sh
.venv/bin/python render_docs.py
```

Edit dimensions at the top of `cad/stand.py` or `cad/magnetic.py`.
The magnetic model also accepts `--usb left|right|up` and `--magnet left|right`;
USB and magnet cannot share a side. The desktop stand currently has fixed right-side USB.

## Folder layout

```text
housing/
  build.py              Build and validate either current model
  render_docs.py        Refresh the front-facing README images
  prepare_artifact.py   Collect both models with PCB-qualified filenames
  cad/                  Models, reference importer, positions and shared rendering
  docs/magnetic.md      Magnetic model dimensions and assembly instructions
  reference/            Original manufacturer STEP and dimension drawing
  output/               Current generated STL, STEP, fit reports and previews (ignored)
  pyproject.toml        CAD and rendering dependencies and PyOpenGL override
  uv.lock               Fully resolved, hashed dependency lock
```

The local `.venv/` is ignored. Old experiments, duplicate renderers and obsolete
exports have been removed; their committed sources remain in Git history.

## Reference model and validation limits

`reference/RP2040-LCD-1.28.step` is the manufacturer board reference.
`cad.board_reference.import_board()` corrects the simplified stack to the measured
**1.6 mm PCB + 2.3 mm display** and adds a conservative envelope over the
USB-side display connector. The connector envelope reaches the display face;
its lateral outline is approximate. The original STEP is retained unchanged.

The current checks require single valid housing/cover solids and watertight
STLs, check the current parts against components and sampled insertion
motions, and extend the USB metal-shell silhouette through the wall to ensure
it is unobstructed. They do not model the cable overmould, loose wires, battery
mating plug, material flex or every intermediate insertion position. The board
reference also contains a pre-existing invalid component solid; a passing
housing check does not certify every imported component.

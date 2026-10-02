# Component references

- `RP2040-LCD-1.28.step`: original manufacturer PCB/display model, preserved
  unchanged. Source: [Waveshare 3D drawing](https://files.waveshare.com/upload/a/a2/RP2040-LCD-1.28-3D-Drawing.zip).
- `board.pdf`: supplied board dimension drawing.

`cad/board_reference.py` corrects the simplified STEP to the user's measured
1.6 mm PCB and 2.3 mm display thickness (3.9 mm total). It also reserves the
USB-side display connector up to the LCD front plane. The connector's lateral
outline is a conservative envelope, not a measured shape.

Original solid 59 is invalid in the manufacturer STEP. Magnetic housing fit
checks use its conservative bounding box; the reference itself stays unchanged.
The generated corrected STEP and drawing screenshots are not required to build.

Battery: MakerFocus 3.7 V 2000 mAh LiPo, nominal 50 × 34 × 10 mm.
Both models retain the selected 52 × 34.8 × 10.8 mm clearance envelope.
Manufacturer dimensions allow ±2 mm, so measure the actual pack before assembly.

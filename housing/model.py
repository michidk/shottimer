"""Phase 1: parametric shape study, not a finished printable enclosure.

Coordinates: X right, Y rear, Z up. Viewer is in front (-Y).
Angle is the upward elevation of the screen normal: 0 = vertical screen.
"""
from pathlib import Path
import argparse
import json
import math
import cadquery as cq

# ============================================================
# PARAMETERS — millimetres unless marked otherwise
# ============================================================
screen_tilt_deg = 20.0  # degrees back from vertical, configurable before printing
base_width = 66.0
base_depth = 50.0
base_height = 20.0
wall = 2.4
lid_thickness = 2.4

# Manufacturer: https://www.makerfocus.com/products/
# makerfocus-3-7v-2000mah-lithium-rechargeable-battery-1s-3c-lipo-battery-with-protection-board-pack-of-4
battery_length = 50.0
battery_width = 34.0
battery_thickness = 10.0
battery_dimension_tolerance = 2.0  # stated ±2 mm on each overall dimension
battery_reserved_length = 56.0   # max pack + 4 mm total clearance/wire allowance
battery_reserved_width = 40.0
battery_reserved_height = 14.0

# Waveshare drawing gives R18.25 PCB; official STEP has a USB tab and headers.
# Source: https://files.waveshare.com/upload/a/a2/RP2040-LCD-1.28-3D-Drawing.zip
# STEP full bounds: 36.524 x 39.512 x 6.500 mm (not just circular PCB diameter).
screen_active_diameter = 32.4
screen_glass_diameter = 35.6
head_diameter = 50.0
head_depth = 14.0
head_front_y = -4.0
head_base_gap = 3.0
neck_width = 24.0
neck_front_y = 1.0
neck_rear_y = 18.0
neck_top_rear_y = 13.0
neck_top_drop = 5.0
joint_overlap = 0.3
preview_face_thickness = 0.3
epsilon = 0.01


def build(tilt=screen_tilt_deg):
    if not 0 <= tilt <= 40:
        raise ValueError('This shape study supports 0–40 degrees back from vertical.')
    angle = math.radians(tilt)
    r = head_diameter / 2
    # Keep the lowest point at the back of the housing above the base at every tilt.
    head_z = base_height + head_base_gap + r*math.cos(angle) + head_depth*math.sin(angle)

    def pose(obj):
        return obj.rotate((0, 0, 0), (1, 0, 0), 90-tilt).translate((0, head_front_y, head_z))

    # Open-top battery tray. Retention and corner rounding belong to later phases.
    tray_top = base_height-lid_thickness
    outer = cq.Workplane('XY').box(base_width, base_depth, tray_top, centered=(True, True, False))
    inner = cq.Workplane('XY').workplane(offset=wall).box(
        base_width-2*wall, base_depth-2*wall, base_height,
        centered=(True, True, False))
    tray = outer.cut(inner)
    lid = cq.Workplane('XY').workplane(offset=tray_top).box(
        base_width, base_depth, lid_thickness, centered=(True, True, False))

    head = pose(cq.Workplane('XY').workplane(offset=-head_depth).circle(r).extrude(head_depth))
    cavity = pose(cq.Workplane('XY').workplane(offset=-head_depth-epsilon)
                  .circle(r-wall).extrude(head_depth-wall+epsilon))
    # Broad support behind the screen; future cable passage will connect both cavities.
    # Follow the rear-bottom rim as tilt increases. A fixed short support would
    # eventually meet only the open cavity, leaving the head disconnected.
    back_bottom_y = head_front_y-r*math.sin(angle)+head_depth*math.cos(angle)
    support_front_y = min(neck_front_y, back_bottom_y-wall)
    neck = cq.Workplane('YZ').polyline([
        (support_front_y, base_height-joint_overlap),
        (neck_rear_y, base_height-joint_overlap),
        (neck_top_rear_y, head_z-neck_top_drop),
        (support_front_y, head_z-neck_top_drop),
    ]).close().extrude(neck_width/2, both=True)
    upper = lid.union(neck).union(head).cut(cavity)

    # Visual references only; these are never included in printable-part exports.
    face = pose(cq.Workplane('XY').circle(screen_glass_diameter/2).extrude(preview_face_thickness))
    active = pose(cq.Workplane('XY').workplane(offset=preview_face_thickness)
                  .circle(screen_active_diameter/2).extrude(preview_face_thickness))
    battery = cq.Workplane('XY').workplane(offset=wall).box(
        battery_length, battery_width, battery_thickness, centered=(True, True, False))
    # Model the maximum specified battery envelope as a separate fit check.
    battery_max = cq.Workplane('XY').workplane(offset=wall+epsilon).box(
        battery_length+battery_dimension_tolerance,
        battery_width+battery_dimension_tolerance,
        battery_thickness+battery_dimension_tolerance,
        centered=(True, True, False))
    assert battery_max.intersect(tray).val().Volume() < 1e-6
    assert battery_reserved_length <= base_width-2*wall
    assert battery_reserved_width <= base_depth-2*wall
    assert battery_reserved_height <= tray_top-wall

    for name, part in [('tray', tray), ('upper', upper)]:
        assert part.val().isValid(), f'{name}: invalid BRep'
        assert len(part.solids().vals()) == 1, f'{name}: disconnected solid'
    assert tray.intersect(upper).val().Volume() < 1e-6

    assembly = cq.Compound.makeCompound([tray.val(), upper.val()])
    bb = assembly.BoundingBox()
    report = dict(stage='Phase 1 / shape study — not ready to print', tilt_deg=tilt,
                  overall_mm=[round(bb.xlen,2),round(bb.ylen,2),round(bb.zlen,2)],
                  battery_nominal_mm=[battery_length,battery_width,battery_thickness],
                  battery_max_mm=[battery_length+2,battery_width+2,battery_thickness+2],
                  battery_reserved_mm=[battery_reserved_length,battery_reserved_width,battery_reserved_height],
                  head_center_z_mm=round(head_z,2), usb_side='right (board rotated 90 degrees)',
                  pending=['screen aperture and retention', 'USB-C plug clearance',
                           'wire passage', 'rear cover', 'fasteners', 'feet and edge finishing',
                           'coffee machine temperature/location and printing material'])
    return dict(tray=tray, upper=upper, face=face, active=active, battery=battery,
                lid=lid, neck=neck, pose=pose,
                assembly=assembly, report=report)


# ============================================================
# EXPORT
# ============================================================
def export(tilt, output):
    parts=build(tilt)
    output=Path(output)
    output.mkdir(parents=True, exist_ok=True)
    for name in ('tray','upper'):
        cq.exporters.export(parts[name], str(output/f'concept_{name}.stl'),
                            tolerance=0.01, angularTolerance=0.1)
    cq.exporters.export(parts['assembly'], str(output/'concept_assembly.step'))
    # Fuse touching parts only for the assembled preview mesh; the STEP and
    # individual part STLs preserve the lid/tray separation.
    cq.exporters.export(parts['tray'].union(parts['upper']), str(output/'concept_assembly.stl'),
                        tolerance=0.01, angularTolerance=0.1)
    (output/'dimensions.json').write_text(json.dumps(parts['report'], indent=2)+'\n')
    print(json.dumps(parts['report']))


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tilt',type=float,default=screen_tilt_deg)
    parser.add_argument('--out',default=str(Path(__file__).parent/'output'))
    args=parser.parse_args()
    export(args.tilt,args.out)

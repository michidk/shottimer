"""Phase 2 assembly study: front-loaded board, removable bezel, side USB.

The manufacturer STEP is the fit reference. This remains a prototype study:
base fasteners, cable-head fit, physical tolerances and print material are pending.
"""
from pathlib import Path
import math
import json
import argparse
import cadquery as cq
import model

# ============================================================
# PARAMETERS — mm
# ============================================================
front_bezel_t = 2.4
screen_opening_d = 33.2       # 32.4 mm active area + 0.4 mm radial clearance
board_glass_front_z = -0.4   # reserve for a compliant perimeter gasket
board_source_center = (18.01, -0.142, 0.0)
source_glass_front_z = -2.0
pcb_rear_z = -2.4           # original PCB component face is source Z=0
support_radius = 16.8
support_pad_r = 1.2
support_angles = (90, 210, 330)
screw_circle_r = 21.6
screw_boss_r = 2.7
screw_angles = (45, 135, 225, 315)
screw_pilot_d = 1.7         # candidate M2 thread-forming screw; tune on a test piece
screw_pilot_depth = 5.6
screw_clearance_d = 2.3
usb_slot_width = 14.0       # cable overmould clearance is still to be measured
usb_slot_back_z = -7.0
usb_slot_start_x = 17.0
usb_slot_length = 15.0
wire_hole_d = 5.0
wire_entry_local = (0.0, -16.0, -10.0)
wire_floor_overlap = 0.1
gasket_od = 35.2
gasket_id = screen_opening_d
insertion_check_distances = (0, 0.5, 1, 2, 4, 6, 10, 15, 25)
ROOT=Path(__file__).parent
OUT=ROOT/'output'/'assembly-study'


def xy_at(radius, angle):
    a=math.radians(angle)
    return radius*math.cos(a),radius*math.sin(a)


MEASURED_PCB_THICKNESS = 1.6
MEASURED_DISPLAY_THICKNESS = 2.3

def import_board(include_display_connector=True, correct_stack=True):
    """Import manufacturer geometry plus the user-confirmed display connector envelope."""
    board = (cq.importers.importStep(str(ROOT/'reference'/'RP2040-LCD-1.28.step'))
            .translate(tuple(-v for v in board_source_center))
            .rotate((0,0,0),(1,1,0),180)
            .translate((0,0,board_glass_front_z+source_glass_front_z)))
    if correct_stack:
        # User-measured stack, superseding the simplified STEP thicknesses.
        # Keep the component-side PCB plane fixed; rear components retain position.
        solids=board.solids().vals()
        pcb_box=solids[0].BoundingBox()
        pcb_back=pcb_box.zmin
        display_back=pcb_back+MEASURED_PCB_THICKNESS
        def resize_z(solid,thickness,new_bottom):
            bounds=solid.BoundingBox()
            factor=thickness/bounds.zlen
            matrix=cq.Matrix([[1,0,0,0],[0,1,0,0],
                              [0,0,factor,new_bottom-factor*bounds.zmin],[0,0,0,1]])
            result=solid.transformGeometry(matrix)
            assert result.isValid()
            return result
        solids[0]=resize_z(solids[0],MEASURED_PCB_THICKNESS,pcb_back)
        solids[116]=resize_z(solids[116],MEASURED_DISPLAY_THICKNESS,display_back)
        board=cq.Workplane('XY').newObject([cq.Compound.makeCompound(solids)])
    if not include_display_connector:
        return board
    # User's physical module has a display connector over the USB-side tab,
    # reaching the LCD front plane. STEP omits it. Reserve the full tab footprint;
    # this is a conservative clearance envelope, not a measured connector shape.
    pcb=board.solids().vals()[0]
    lcd=board.solids().vals()[116]
    bottom=pcb.BoundingBox().zmax
    top=lcd.BoundingBox().zmax
    x0,x1=15.5,21.262
    y1=6.404
    y0=y1+(x1-x0)*(9.187-6.404)/(21.262-15.781)
    connector=(cq.Workplane('XY').workplane(offset=bottom)
               .polyline([(x0,-y0),(x1,-y1),(x1,y1),(x0,y0)])
               .close().extrude(top-bottom))
    connector=connector.cut(cq.Workplane('XY').newObject([lcd]))
    return cq.Workplane('XY').newObject([cq.Compound.makeCompound(
        board.solids().vals()+connector.solids().vals())])


def build(tilt=model.screen_tilt_deg, include_board=True):
    base=model.build(tilt)
    pose=base['pose']
    r=model.head_diameter/2
    floor_z=-model.head_depth+model.wall

    # The cup is closed at the rear and completely open at the front before assembly.
    outer=cq.Workplane('XY').workplane(offset=-model.head_depth).circle(r).extrude(model.head_depth)
    inner=cq.Workplane('XY').workplane(offset=floor_z).circle(r-model.wall).extrude(-floor_z+model.epsilon)
    local_body=outer.cut(inner)
    screw_points=[xy_at(screw_circle_r,a) for a in screw_angles]
    bosses=cq.Workplane('XY').workplane(offset=floor_z).pushPoints(screw_points).circle(screw_boss_r).extrude(-floor_z)
    pads=cq.Workplane('XY').workplane(offset=floor_z).pushPoints(
        [xy_at(support_radius,a) for a in support_angles]).circle(support_pad_r).extrude(pcb_rear_z-floor_z)
    local_body=local_body.union(bosses).union(pads)
    pilots=cq.Workplane('XY').workplane(offset=-screw_pilot_depth).pushPoints(
        screw_points).circle(screw_pilot_d/2).extrude(screw_pilot_depth+model.epsilon)
    # Slot reaches the front face: the connector can descend with the board.
    usb_cut=cq.Workplane('XY').workplane(offset=usb_slot_back_z).center(
        usb_slot_start_x+usb_slot_length/2,0).box(usb_slot_length,usb_slot_width,
        -usb_slot_back_z+model.epsilon,centered=(True,True,False))
    local_body=local_body.cut(pilots).cut(usb_cut)

    bezel=cq.Workplane('XY').circle(r).circle(screen_opening_d/2).extrude(front_bezel_t)
    bezel=bezel.cut(cq.Workplane('XY').workplane(offset=-model.epsilon).pushPoints(
        screw_points).circle(screw_clearance_d/2).extrude(front_bezel_t+2*model.epsilon))
    gasket=cq.Workplane('XY').workplane(offset=board_glass_front_z).circle(
        gasket_od/2).circle(gasket_id/2).extrude(-board_glass_front_z)

    # Connect the head to the stand. Recap the cavity after union so the neck
    # cannot occupy the board's insertion corridor.
    upper=base['lid'].union(base['neck']).union(pose(local_body))
    # Only remove the cavity from the neck, preserving cup bosses and support pads.
    neck_trimmed=base['neck'].cut(pose(inner))
    upper=base['lid'].union(neck_trimmed).union(pose(local_body))

    entry=pose(cq.Workplane('XY').newObject([cq.Vertex.makeVertex(*wire_entry_local)])).val().Center()
    wire_start=model.base_height-model.lid_thickness-wire_floor_overlap
    wire=cq.Workplane('XY').workplane(offset=wire_start).center(entry.x,entry.y).circle(
        wire_hole_d/2).extrude(entry.z-wire_start)
    upper=upper.cut(wire)

    for name,part in [('upper',upper),('bezel',bezel)]:
        assert part.val().isValid(),name
        assert len(part.solids().vals())==1,f'{name}: disconnected'
    board=import_board() if include_board else None
    return dict(base=base, pose=pose, upper=upper, bezel=bezel, local_body=local_body,
                board=board, gasket=gasket, wire=wire, tilt=tilt, screw_points=screw_points)


def verify_and_export(tilt=model.screen_tilt_deg):
    parts=build(tilt)
    OUT.mkdir(parents=True,exist_ok=True)
    board=parts['board']
    overlap={}
    # Test the whole populated STEP model, not a cylindrical board approximation.
    # Check the complete world-space body, including neck and base lid.
    for travel in insertion_check_distances:
        moved=parts['pose'](board.translate((0,0,travel)))
        volume=parts['upper'].intersect(moved).val().Volume()
        overlap[str(travel)]=round(volume,8)
        assert volume<1e-5,f'Board insertion blocked at {travel} mm: {volume} mm3'
    bezel_overlap=board.intersect(parts['bezel']).val().Volume()
    assert bezel_overlap<1e-5,'Bezel contacts the module solid'
    # Screw axes stay outside the display module.
    for x,y in parts['screw_points']:
        axis=cq.Workplane('XY').workplane(offset=-screw_pilot_depth).center(x,y).circle(
            screw_clearance_d/2).extrude(screw_pilot_depth+front_bezel_t)
        assert axis.intersect(board).val().Volume()<1e-5

    bodies={'upper_body':parts['upper'], 'front_bezel':parts['bezel'],
            'battery_tray':parts['base']['tray']}
    for name,body in bodies.items():
        cq.exporters.export(body,str(OUT/f'{name}.stl'),tolerance=.01,angularTolerance=.1)
    assy=cq.Assembly()
    assy.add(parts['upper'],name='upper_body')
    assy.add(parts['pose'](parts['bezel']),name='front_bezel')
    assy.add(parts['base']['tray'],name='battery_tray')
    assy.add(parts['pose'](board),name='Waveshare_reference_only')
    assy.export(str(OUT/'assembly_with_board.step'))
    report=dict(stage='Assembly study — not final print files',tilt_deg=tilt,
                insertion_overlap_mm3=overlap,bezel_overlap_mm3=round(bezel_overlap,8),
                insertion_direction='From the front, screen facing outward',
                retention='Four candidate M2 x 6 mm thread-forming screws; pilot fit requires test',
                support_angles_deg=list(support_angles),support_radius_mm=support_radius,
                bezel_opening_mm=screen_opening_d,usb_slot_mm=[usb_slot_width,-usb_slot_back_z],
                gasket='0.4 mm assembled thickness; soft perimeter pad, no pressure on active area',
                pending=['physical component and cable measurements', 'base fastening',
                         'wire and plug routing check with real harness', 'material and temperature',
                         'print orientation and final finishing'])
    (OUT/'fit-check.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report))


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tilt',type=float,default=model.screen_tilt_deg)
    args=parser.parse_args()
    verify_and_export(args.tilt)

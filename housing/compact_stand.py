"""Rounded-top enclosure: front-loaded display, upright battery behind it.

This is a shape and assembly study, not final print-ready hardware.
Screen and battery tilt are independently configurable before printing.
"""
from pathlib import Path
import argparse
import json
import math
import cadquery as cq
import trimesh
from assembly_study import import_board, xy_at

# ============================================================
# PARAMETERS — millimetres, angles back from vertical
# ============================================================
screen_tilt_deg = 20.0
battery_tilt_deg = 0.0
body_width = 54.0
front_base_y = -17.5
wall = 2.4
edge_chamfer = 2.0
screen_center_z = 47.5  # raised slightly to clear the upright pack under the tilted crown
bottom_lid_t = 2.0
part_clearance = 0.2

# Manufacturer pack size: 50 x 34 x 10, +/-2 mm on overall dimensions.
# https://www.makerfocus.com/products/makerfocus-3-7v-2000mah-lithium-rechargeable-battery-1s-3c-lipo-battery-with-protection-board-pack-of-4
battery_nominal = (34.0, 50.0, 10.0)  # local X width, local Y length, local Z thickness
battery_maximum = (36.0, 52.0, 12.0)
battery_center_z = 29.0  # max pack bottom is 3 mm, above the 2 mm bottom cover
battery_component_gap = 1.0
battery_case_gap = 0.8
battery_search_start_y = 10.0
battery_search_step = 0.5
battery_search_steps = 90

# Source: Waveshare RP2040-LCD-1.28 STEP stored in reference/.
bezel_radius = 24.5
bezel_t = 2.4
bezel_fit_gap = 0.15
screen_opening_d = 33.2
insertion_radius = 22.6
mount_outer_radius = 25.5
mount_rear_z = -8.0
mount_front_overlap = 0.1
board_extra_recess = -bezel_t
pcb_back_z = -2.4 + board_extra_recess
screen_glass_z = -0.4 + board_extra_recess
support_radius = 16.8
support_half_width = 1.2
support_t = 0.8
support_angles = (90, 210, 300)  # keep the lower-right guide away from the USB-tab flare
guide_inner_radius = 18.65
guide_radial_length = 2.4
guide_width = 3.0
guide_height = 3.2
screw_radius = 21.6
screw_boss_r = 2.7
screw_angles = (45, 135, 225, 315)
screw_pilot_d = 1.7  # prototype pilot for M2 thread-forming screws; needs material test
screw_clearance_d = 2.3
usb_width = 14.0    # provisional cable overmould allowance
usb_back_z = -10.0
usb_front_z = -2.6
usb_start_x = 17.0
usb_cut_length = 20.0
gasket_od = 35.2
epsilon = 0.01
insertion_samples = (0, 0.5, 1, 2, 4, 6, 10, 15, 25, 40)
battery_insertion_samples = (0, 2, 5, 10, 20, 35, 50, 70)
ROOT=Path(__file__).parent
OUT=ROOT/'output'/'angled-rounded'


def overlap(a,b):
    return a.intersect(b).val().Volume()


def build(screen_tilt=screen_tilt_deg, battery_tilt=battery_tilt_deg):
    if not 5 <= screen_tilt <= 35 or not 0 <= battery_tilt <= 25:
        raise ValueError('Study range: screen 5–35 degrees, battery 0–25 degrees.')
    a=math.radians(screen_tilt)
    front_y=front_base_y+screen_center_z*math.tan(a)
    crown_radius=body_width/2
    height=screen_center_z+crown_radius
    def pose(obj):
        return obj.rotate((0,0,0),(1,0,0),90-screen_tilt).translate((0,front_y,screen_center_z))
    board_local=import_board().translate((0,0,board_extra_recess))
    board=pose(board_local)

    def pack(dims, y):
        return (cq.Workplane('XY').box(*dims)
                .rotate((0,0,0),(1,0,0),90-battery_tilt)
                .translate((0,y,battery_center_z)))
    # Pack behind the actual populated module, maintaining a component gap.
    for i in range(battery_search_steps):
        battery_y=battery_search_start_y+i*battery_search_step
        maximum=pack(battery_maximum,battery_y)
        distance=maximum.val().distance(board.val())
        if distance>=battery_component_gap and overlap(maximum,board)<1e-6:
            break
    else:
        raise ValueError('No battery placement meets the component clearance.')
    battery=pack(battery_nominal,battery_y)
    bb=maximum.val().BoundingBox()
    if bb.zmin < bottom_lid_t+part_clearance:
        raise ValueError('Battery tilt requires a higher battery_center_z.')
    rear_y=max(bb.ymax+wall+battery_case_gap,
               front_base_y+height*math.tan(a)+2*wall+edge_chamfer)
    depth=rear_y-front_base_y

    # The circular crown shares the display's local plane and center. Extruding
    # along the display normal makes its roof fall toward the rear at screen_tilt.
    def arch(radius):
        local=(cq.Workplane('XY').moveTo(-radius,-2*height)
               .lineTo(radius,-2*height).lineTo(radius,0)
               .threePointArc((0,radius),(-radius,0))
               .close().extrude(depth+height,both=True))
        return pose(local)

    # Chamfer the clean exterior before making openings.
    wedge=(cq.Workplane('YZ').polyline([(front_base_y,0),(rear_y,0),
           (rear_y,height),(front_base_y+height*math.tan(a),height)])
           .close().extrude(body_width/2,both=True))
    outer=wedge.intersect(arch(crown_radius)).edges().chamfer(edge_chamfer)
    inner_front=lambda z: front_base_y+z*math.tan(a)+wall/math.cos(a)
    inner=(cq.Workplane('YZ').polyline([(inner_front(-epsilon),-epsilon),
           (rear_y-wall,-epsilon),(rear_y-wall,height-wall),
           (inner_front(height-wall),height-wall)])
           .close().extrude(body_width/2-wall,both=True))
    inner=inner.intersect(arch(crown_radius-wall))
    body=outer.cut(inner)

    opening=cq.Workplane('XY').workplane(offset=mount_rear_z-epsilon).circle(
        insertion_radius).extrude(-mount_rear_z+2*epsilon)
    recess=cq.Workplane('XY').workplane(offset=-bezel_t).circle(
        bezel_radius+bezel_fit_gap).extrude(bezel_t+epsilon)
    body=body.cut(pose(opening)).cut(pose(recess))

    # Internal frame ties into the front wall. The battery shares the main cavity;
    # there is no thick rear wall behind the display module.
    frame=(cq.Workplane('XY').workplane(offset=mount_rear_z)
           .circle(mount_outer_radius).circle(insertion_radius)
           .extrude(-bezel_t-mount_rear_z+mount_front_overlap))
    screw_points=[xy_at(screw_radius,angle) for angle in screw_angles]
    bosses=(cq.Workplane('XY').workplane(offset=mount_rear_z)
            .pushPoints(screw_points).circle(screw_boss_r).extrude(-bezel_t-mount_rear_z))
    frame=frame.union(bosses)
    for angle in support_angles:
        radial_start=support_radius-support_half_width
        arm_length=mount_outer_radius-radial_start
        arm=(cq.Workplane('XY').workplane(offset=pcb_back_z-support_t)
             .center(radial_start+arm_length/2,0)
             .box(arm_length,2*support_half_width,support_t,centered=(True,True,False))
             .rotate((0,0,0),(0,0,1),angle))
        guide=(cq.Workplane('XY').workplane(offset=pcb_back_z-support_t)
               .center(guide_inner_radius+guide_radial_length/2,0)
               .box(guide_radial_length,guide_width,guide_height,centered=(True,True,False))
               .rotate((0,0,0),(0,0,1),angle))
        frame=frame.union(arm).union(guide)
    pilots=(cq.Workplane('XY').workplane(offset=mount_rear_z-epsilon)
            .pushPoints(screw_points).circle(screw_pilot_d/2)
            .extrude(-bezel_t-mount_rear_z+2*epsilon))
    frame=frame.cut(pilots)
    body=body.union(pose(frame)).intersect(outer)
    # Recut the recess after adding the mounting frame for a flush bezel seat.
    body=body.cut(pose(recess))
    usb=(cq.Workplane('XY').workplane(offset=usb_back_z)
         .center(usb_start_x+usb_cut_length/2,0)
         .box(usb_cut_length,usb_width,usb_front_z-usb_back_z,centered=(True,True,False)))
    body=body.cut(pose(usb))

    bezel=(cq.Workplane('XY').workplane(offset=-bezel_t)
           .circle(bezel_radius).circle(screen_opening_d/2).extrude(bezel_t))
    bezel=bezel.cut(cq.Workplane('XY').workplane(offset=-bezel_t-epsilon)
                   .pushPoints(screw_points).circle(screw_clearance_d/2).extrude(bezel_t+2*epsilon))
    gasket=(cq.Workplane('XY').workplane(offset=screen_glass_z)
            .circle(gasket_od/2).circle(screen_opening_d/2).extrude(-bezel_t-screen_glass_z))
    lid_front=inner_front(bottom_lid_t)+part_clearance
    lid_rear=rear_y-wall-part_clearance
    lid=(cq.Workplane('XY').center(0,(lid_front+lid_rear)/2)
         .box(body_width-2*wall-2*part_clearance,lid_rear-lid_front,bottom_lid_t,
              centered=(True,True,False)))
    # Visual active display surface, sized from Waveshare's 32.4 mm specification.
    active=pose(cq.Workplane('XY').workplane(offset=screen_glass_z+epsilon)
                .circle(32.4/2).extrude(epsilon))
    for name,part in [('body',body),('bezel',bezel),('lid',lid)]:
        assert part.val().isValid(),f'{name}: invalid BRep'
        assert len(part.solids().vals())==1,f'{name}: disconnected'
    bounds=body.val().BoundingBox()
    return dict(body=body,outer=outer,bezel=pose(bezel),bezel_local=bezel,lid=lid,
                board=board,board_local=board_local,battery=battery,battery_max=maximum,
                gasket=pose(gasket),active=active,pose=pose,
                parameters=dict(screen_tilt_deg=screen_tilt,battery_tilt_deg=battery_tilt,
                    dimensions_mm=[round(bounds.xlen,2),round(bounds.ylen,2),round(bounds.zlen,2)],
                    top_profile='semicircular crown aligned with screen', crown_radius_mm=crown_radius,
                    roof_rearward_slope_deg=screen_tilt,
                    battery_center_y_mm=battery_y,
                    battery_to_board_gap_mm=round(distance,3),
                    footprint_reduction_percent=round(100*(1-bounds.xlen*bounds.ylen/(66*50)),1)))


def verify(parts):
    body=parts['body']
    report=dict(stage='Compact shape and assembly study — not final print files',**parts['parameters'])
    report['display_insertion_overlap_mm3']={}
    for travel in insertion_samples:
        board=parts['pose'](parts['board_local'].translate((0,0,travel)))
        v=overlap(body,board)
        assert v<1e-5,f'Display insertion collision at {travel}: {v}'
        report['display_insertion_overlap_mm3'][str(travel)]=round(v,8)
    report['battery_insertion_overlap_mm3']={}
    for travel in battery_insertion_samples:
        pack=parts['battery_max'].translate((0,0,-travel))
        v=overlap(body,pack)+overlap(pack,parts['board'])
        assert v<1e-5,f'Battery insertion collision at {travel}: {v}'
        report['battery_insertion_overlap_mm3'][str(travel)]=round(v,8)
    for name in ('bezel','lid'):
        assert overlap(body,parts[name])<1e-5,f'{name} / body collision'
        assert overlap(parts[name],parts['board'])<1e-5,f'{name} / board collision'
        assert overlap(parts[name],parts['battery_max'])<1e-5,f'{name} / battery collision'
    report['pending']=['bottom lid fastening and battery restraint',
                       'physical fit and USB cable overmould measurements',
                       'wiring and connector polarity check',
                       'printer, material, machine surface temperature',
                       'print orientation and final finishing']
    return report


def main(screen_tilt,battery_tilt):
    parts=build(screen_tilt,battery_tilt)
    OUT.mkdir(parents=True,exist_ok=True)
    report=verify(parts)
    assembly=cq.Assembly()
    for name in ('body','bezel','lid','board','battery'):
        assembly.add(parts[name],name=name)
    assembly.export(str(OUT/'compact_assembly.step'))
    report['mesh_checks']={}
    for name in ('body','bezel_local','lid'):
        path=OUT/f'{name}.stl'
        cq.exporters.export(parts[name],str(path),tolerance=.01,angularTolerance=.1)
        mesh=trimesh.load(path,force='mesh')
        assert mesh.is_watertight and mesh.is_volume,str(path)
        report['mesh_checks'][name]={'watertight':True,'volume_mm3':round(float(mesh.volume),2)}
    (OUT/'fit-check.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report),flush=True)
    return parts,report


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--screen-tilt',type=float,default=screen_tilt_deg)
    p.add_argument('--battery-tilt',type=float,default=battery_tilt_deg)
    p.add_argument('--render',action='store_true',help='Render this exact checked model without rebuilding it.')
    args=p.parse_args()
    parts,report=main(args.screen_tilt,args.battery_tilt)
    if args.render:
        from render_compact import main as render_main
        render_main(parts,report)

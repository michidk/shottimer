"""Phase 1 shape study, NOT a functional print release.

X right, Y rear, Z up; sides viewed from display. Surface markers are
illustrations, not openings. Board/battery use existing measured references.
"""
from pathlib import Path
import argparse
import json
import cadquery as cq
import trimesh
from assembly_study import import_board

# PARAMETERS — mm
width, height, depth = 48.0, 58.0, 28.0
wall, corner_radius = 2.4, 6.0
cover_thickness, seam_gap = 2.4, 0.2
screen_diameter, board_recess = 33.2, -2.85
battery_allowance = (34.8, 10.8, 52.0)  # same as current stand
battery_center_y = 6.0
magnet_diameter, magnet_thickness = 12.0, 2.7  # user-confirmed disc
marker_thickness = 0.3
usb_width, usb_depth = 12.0, 7.0
usb_depth_center = -3.56952175415 + board_recess
USB_ANGLES = {'right': 0, 'up': 90, 'left': 180}
OUT = Path(__file__).parent / 'output' / 'magnetic-concept'

# MODEL — base shell only, features await shape review

def validate_positions(usb, magnet):
    if usb not in USB_ANGLES:
        raise ValueError('USB position must be left, right, or up')
    if magnet not in ('left', 'right'):
        raise ValueError('Magnet position must be left or right')
    if usb == magnet:
        raise ValueError('USB and magnet cannot occupy the same side')


def build(usb='left', magnet='right'):
    validate_positions(usb, magnet)
    def rounded_box(w, d, h, r):
        return cq.Workplane('XY').box(w, d, h).edges('|Y').fillet(r)
    outer = rounded_box(width, depth, height, corner_radius).translate((0,0,height/2))
    split = depth/2-cover_thickness
    cavity = rounded_box(width-2*wall, depth, height-2*wall, corner_radius-wall).translate((0,wall,height/2))
    front = cq.Workplane('XY').box(width*2,depth*2,height*2).translate((0,split-seam_gap/2-depth,height/2))
    rear = cq.Workplane('XY').box(width*2,depth*2,height*2).translate((0,split+seam_gap/2+depth,height/2))
    body, cover = outer.cut(cavity).intersect(front), outer.intersect(rear)
    def pose(shape):
        return shape.rotate((0,0,0),(1,0,0),90).translate((0,-depth/2,height/2))
    board = pose(import_board().translate((0,0,board_recess)).rotate((0,0,0),(0,0,1),USB_ANGLES[usb]))
    battery = cq.Workplane('XY').box(*battery_allowance).translate((0,battery_center_y,height/2))
    active = pose(cq.Workplane('XY').circle(screen_diameter/2).extrude(marker_thickness))
    sign = 1 if magnet == 'right' else -1
    magnet_marker = cq.Workplane('YZ').circle(magnet_diameter/2).extrude(sign*marker_thickness).translate((sign*width/2,0,height/2))
    if usb == 'up':
        port = cq.Workplane('XY').box(usb_width,usb_depth,marker_thickness).translate((0,-depth/2-usb_depth_center,height+marker_thickness/2))
    else:
        sign = 1 if usb == 'right' else -1
        port = cq.Workplane('XY').box(marker_thickness,usb_depth,usb_width).translate((sign*(width/2+marker_thickness/2),-depth/2-usb_depth_center,height/2))
    return dict(body=body,cover=cover,board=board,battery=battery,active=active,magnet=magnet_marker,port=port)

# EXPORT — concept-only STL; front-face-down orientation when printing a sample

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--usb', choices=USB_ANGLES, default='left')
    parser.add_argument('--magnet', choices=('left','right'), default='right')
    args = parser.parse_args()
    parts = build(args.usb,args.magnet)
    output = OUT / f'usb-{args.usb}_magnet-{args.magnet}'
    output.mkdir(parents=True,exist_ok=True)
    for name in ('body','cover'):
        shape = parts[name]
        assert shape.val().isValid() and len(shape.solids().vals()) == 1, name
        path = output / f'{name}_concept.stl'
        cq.exporters.export(shape,str(path),tolerance=.01,angularTolerance=.1)
        mesh = trimesh.load(path,force='mesh')
        assert mesh.is_watertight and mesh.is_volume, name
    # Uncut front deliberately intersects LCD until seating relief is added.
    for name in ('body','cover','board'):
        volume = parts[name].intersect(parts['battery']).val().Volume()
        assert volume < 1e-5, (name,'battery overlap',volume)
    report = {'stage':'base shape only; no openings, magnet pocket or retention',
              'dimensions_width_depth_height_mm':[width,depth,height],
              'usb_position':args.usb,'magnet_position':args.magnet,
              'magnet_diameter_thickness_mm':[magnet_diameter,magnet_thickness]}
    (output/'shape-check.json').write_text(json.dumps(report,indent=2)+'\n')
    from render_magnetic_concept import render_preview
    render_preview(parts,output/'preview.png',args.usb,args.magnet)
    print(output)

if __name__ == '__main__':
    main()

"""Magnetic housing feature prototype; sides named looking at the display.

X right, Y rear, Z up. Body prints front-face-down, cover outside-face-down.
Reuses the user-measured board stack and battery allowance of the current stand.
"""
from pathlib import Path
import json
import cadquery as cq
import trimesh
from .board_reference import board_part, import_board
from .positions import validate_positions, USB_ANGLES

# ============================================================
# PARAMETERS — mm unless marked degrees
# ============================================================
width, height, depth = 48.0, 58.0, 28.0
wall, corner_radius, chamfer = 2.4, 6.0, 2.0
cover_thickness, seam_gap = 2.4, 0.2
screen_diameter, recess_radius, front_lip = 33.2, 18.4, 1.2
board_recess, pcb_back, pcb_gap = -2.85, -5.25, .15
# Raise the board for top USB to preserve the side version's 2.92 mm setback.
top_board_raise = (height-width)/2
usb_width, usb_depth, usb_radius = 12.0, 7.0, .8
usb_depth_center = -3.56952175415 + board_recess
usb_cut_start, usb_cut_length = 17.0, 20.0
connector_outline = [(15.2,-9.6),(21.55,-6.7),(21.55,6.7),(15.2,9.6)]
guide_angles = (60,90,150,210,270,300)
guide_length, guide_width, guide_back = 2.4, 3.0, -5.15
# Same press-fit recipe as stand.py.
rim_depth, rim_wall, rim_clearance = 3.5, 1.4, .15
rim_rib_projection, rim_lead_chamfer = .20, .4
rim_root_overlap, rib_height = .4, 4.0
rib_z_positions = (12.0,46.0)
notch_width, notch_depth, notch_height = 10.0, 1.0, 1.0
battery_allowance = (34.8,10.8,52.0)
battery_nominal = (34.0,10.0,50.0)
battery_center_y, battery_bottom = 6.0, 3.0
battery_wall, battery_gap, battery_shelf_t = 1.2, .2, 1.2
battery_lip_height = 2.4
battery_rail_z = (10.0,44.0)
battery_rail_height = 3.0
# Rear PCB stops follow the current stand's contact locations in board coordinates.
retainer_points = ((-18.0,0.0,2.0),(19.3,-6.4,1.4))
top_usb_retainer_points = ((0.0,-18.0,1.0),(0.0,18.0,1.0))
retainer_tip_width, retainer_depth = 1.0, 1.6
retainer_arm_x, retainer_arm_width = 18.85, 1.2
magnet_diameter, magnet_thickness = 12.0, 2.7
magnet_diameter_clearance, magnet_glue_depth = .5, .5
magnet_backing, magnet_border = 1.2, 1.2
magnet_center_y, magnet_center_z = 0.0, height/2
travel_samples = (0,.5,1,2,3.5,5,10,20,40,60)
eps, volume_tolerance = .01, 1e-5
OUT = Path(__file__).resolve().parents[1]/'output'/'magnetic'


def box(w,d,h,center):
    return cq.Workplane('XY').box(w,d,h).translate(center)


def rounded_box(w,d,h,r):
    return cq.Workplane('XY').box(w,d,h).edges('|Y').fillet(r)


def overlap(a,b):
    # Disjoint bounds are exactly non-overlapping and need no OCC boolean.
    aa, bb = a.val().BoundingBox(), b.val().BoundingBox()
    if any(getattr(aa,axis+'max') <= getattr(bb,axis+'min') or
           getattr(bb,axis+'max') <= getattr(aa,axis+'min') for axis in 'xyz'):
        return 0.0
    aa_solids, bb_solids = a.solids().vals(), b.solids().vals()
    if len(aa_solids) > 1 or len(bb_solids) > 1:
        return sum(overlap(cq.Workplane('XY').newObject([sa]),
                           cq.Workplane('XY').newObject([sb]))
                   for sa in aa_solids for sb in bb_solids)
    return a.intersect(b).val().Volume()


# ============================================================
# MODEL
# ============================================================
def build(usb='left', magnet='right'):
    validate_positions(usb,magnet)
    angle = USB_ANGLES[usb]
    screen_z = height/2 + (top_board_raise if usb == 'up' else 0)
    def rotate(shape):
        return shape.rotate((0,0,0),(0,0,1),angle)
    def pose(shape):
        return shape.rotate((0,0,0),(1,0,0),90).translate((0,-depth/2,screen_z))
    def board_pose(shape):
        return pose(rotate(shape))

    raw_outer = rounded_box(width,depth,height,corner_radius).translate((0,0,height/2))
    # Match the desktop stand's exterior bevel while preserving the flat base.
    chamfer_edges = [e for e in raw_outer.edges().vals() if e.BoundingBox().zmax > eps]
    outer = raw_outer.newObject(chamfer_edges).chamfer(chamfer)
    split = depth/2-cover_thickness
    root = split+seam_gap/2
    cavity = rounded_box(width-2*wall,depth,height-2*wall,corner_radius-wall).translate((0,wall,height/2))
    front = box(width*2,depth*2,height*2,(0,split-seam_gap/2-depth,height/2))
    rear = box(width*2,depth*2,height*2,(0,root+depth,height/2))
    body, cover = outer.cut(cavity).intersect(front), outer.intersect(rear)
    body = body.cut(pose(cq.Workplane('XY').workplane(offset=-depth).circle(screen_diameter/2).extrude(depth*2)))
    body = body.cut(pose(cq.Workplane('XY').workplane(offset=-depth).circle(recess_radius).extrude(depth-front_lip)))
    tab = cq.Workplane('XY').workplane(offset=-depth).polyline(connector_outline).close().extrude(depth-front_lip)
    body = body.cut(board_pose(tab))
    for a in guide_angles:
        guide = box(guide_length,guide_width,-front_lip-guide_back,
                    (recess_radius+guide_length/2,0,(guide_back-front_lip)/2)).rotate((0,0,0),(0,0,1),a)
        body = body.union(board_pose(guide))
    port = box(usb_cut_length,usb_width,usb_depth,
               (usb_cut_start+usb_cut_length/2,0,usb_depth_center)).edges('|X').fillet(usb_radius)
    body = body.cut(board_pose(port))

    # A chamfered locating rim, rooted in the backplate.
    rim_w, rim_h = width-2*(wall+rim_clearance), height-2*(wall+rim_clearance)
    rim_r = corner_radius-wall-rim_clearance
    rim_outer = rounded_box(rim_w,rim_depth+rim_root_overlap,rim_h,rim_r).translate(
        (0,root+(rim_root_overlap-rim_depth)/2,height/2))
    rim_inner = rounded_box(rim_w-2*rim_wall,rim_depth+2*rim_root_overlap,rim_h-2*rim_wall,
                            rim_r-rim_wall).translate((0,root-rim_depth/2,height/2))
    rim = rim_outer.cut(rim_inner)
    leading = [e for e in rim.edges().vals() if e.BoundingBox().ymax < root-rim_depth+eps]
    rim = rim.newObject(leading).chamfer(rim_lead_chamfer)
    # Tall battery needs central breaks at BOTH ends of the rim; sides and
    # rounded corners remain full thickness and all segments root in the cover.
    pocket_width = battery_allowance[0]+2*(battery_gap+battery_wall)
    for z in (0,height):
        rim = rim.cut(box(pocket_width+2*battery_gap,depth,2*(wall+rim_wall+rim_clearance),
                          (0,root,z)))
    cover = cover.union(rim)
    ribs = []
    xface = width/2-wall-rim_clearance
    for sign in (-1,1):
        for z in rib_z_positions:
            rib = (cq.Workplane('XY').workplane(offset=z-rib_height/2)
                   .polyline([(sign*(xface-eps),root-eps),
                              (sign*(xface+rim_rib_projection),root-eps),
                              (sign*(xface+rim_rib_projection),root-rim_depth+2*rim_lead_chamfer),
                              (sign*(xface-eps),root-rim_depth+rim_lead_chamfer)])
                   .close().extrude(rib_height))
            ribs.append(rib)
            cover = cover.union(rib)

    pack_front = battery_center_y-battery_allowance[1]/2
    pack_rear = battery_center_y+battery_allowance[1]/2
    shelf_front = pack_front-battery_gap-battery_wall
    shelf_top = battery_bottom-battery_gap
    shelf_bottom = shelf_top-battery_shelf_t
    shelf = box(pocket_width,depth/2-shelf_front,battery_shelf_t,
                (0,(depth/2+shelf_front)/2,(shelf_bottom+shelf_top)/2))
    # Sink the shelf into the floor, keeping 1.4 mm of solid body below it.
    recess_bottom = shelf_bottom-battery_gap
    recess_height = shelf_top+battery_gap-recess_bottom
    body = body.cut(box(pocket_width+2*battery_gap,depth,recess_height,
                        (0,shelf_front-battery_gap+depth/2,recess_bottom+recess_height/2)))
    # The cut above is a narrow floor recess only, not a full-height side cut.
    lip = box(pocket_width,battery_wall,battery_lip_height,
              (0,shelf_front+battery_wall/2,shelf_top+battery_lip_height/2))
    backing_front = pack_rear+battery_gap
    backing = box(pocket_width,depth/2-backing_front,battery_allowance[2],
                  (0,(backing_front+depth/2)/2,battery_bottom+battery_allowance[2]/2))
    cover = cover.union(shelf).union(lip).union(backing)
    for sign in (-1,1):
        for z in battery_rail_z:
            rail = box(battery_wall,depth/2-pack_front,battery_rail_height,
                       (sign*(battery_allowance[0]/2+battery_gap+battery_wall/2),
                        (depth/2+pack_front)/2,z))
            cover = cover.union(rail)

    # Two rear stops and the front seat capture the PCB; omit the long third L finger.
    # Bridges stay ahead of the battery, and arms run outside it.
    stop_points = top_usb_retainer_points if usb == 'up' else retainer_points
    stops = []
    for x,y,span in stop_points:
        tip = board_pose(box(retainer_tip_width,span,retainer_depth,
                             (x,y,pcb_back-pcb_gap-retainer_depth/2)))
        stops.append(tip)
        bounds = tip.val().BoundingBox()
        cx,cy,cz = bounds.center.toTuple()
        sign = 1 if cx >= 0 else -1
        arm_x = sign*retainer_arm_x
        bridge_left = min(bounds.xmin,arm_x-retainer_arm_width/2)
        bridge_right = max(bounds.xmax,arm_x+retainer_arm_width/2)
        bridge = box(bridge_right-bridge_left,retainer_arm_width,retainer_arm_width,
                     ((bridge_left+bridge_right)/2,bounds.ymax-retainer_arm_width/2,cz))
        arm = box(retainer_arm_width,depth/2-bounds.ymax+retainer_arm_width,retainer_arm_width,
                  (arm_x,(depth/2+bounds.ymax-retainer_arm_width)/2,cz))
        cover = cover.union(tip).union(bridge).union(arm)

    sign = 1 if magnet == 'right' else -1
    pocket_depth = magnet_thickness+magnet_glue_depth
    pocket_d = magnet_diameter+magnet_diameter_clearance
    boss_depth = pocket_depth+magnet_backing
    boss = box(boss_depth,pocket_d+2*magnet_border,pocket_d+2*magnet_border,
               (sign*(width/2-boss_depth/2),magnet_center_y,magnet_center_z)).edges('|X').fillet(magnet_border)
    body = body.union(boss)
    bore = (cq.Workplane('YZ').circle(pocket_d/2).extrude(sign*(pocket_depth+eps))
            .translate((sign*(width/2-pocket_depth),magnet_center_y,magnet_center_z)))
    body = body.cut(bore)
    magnet_shape = (cq.Workplane('YZ').circle(magnet_diameter/2).extrude(sign*magnet_thickness)
                    .translate((sign*(width/2-magnet_thickness),magnet_center_y,magnet_center_z)))
    notch = box(notch_width,notch_depth+eps,notch_height+eps,
                (0,depth/2-notch_depth/2+eps/2,notch_height/2-eps/2))
    cover = cover.cut(notch)
    board_local = import_board().translate((0,0,board_recess))
    board = board_pose(board_local)
    fit_solids, invalid_reference_solids = [], []
    for index,solid in enumerate(board.solids().vals()):
        if not solid.isValid():
            bounds = solid.BoundingBox()
            solid = box(bounds.xlen,bounds.ylen,bounds.zlen,bounds.center.toTuple()).val()
            invalid_reference_solids.append(index)
        fit_solids.append(solid)
    board_fit = cq.Workplane('XY').newObject([cq.Compound.makeCompound(fit_solids)])
    socket = board_part(board_local,'usb_shell').BoundingBox()
    usb_passage = board_pose(box(usb_cut_start+usb_cut_length-socket.xmin,
                                 socket.ylen,socket.zlen,
                                 ((usb_cut_start+usb_cut_length+socket.xmin)/2,
                                  (socket.ymin+socket.ymax)/2,(socket.zmin+socket.zmax)/2)))
    battery_max = box(*battery_allowance,(0,battery_center_y,battery_bottom+battery_allowance[2]/2))
    battery = box(*battery_nominal,(0,battery_center_y,battery_bottom+battery_nominal[2]/2))
    active = pose(cq.Workplane('XY').workplane(offset=-1.35+eps).circle(16.2).extrude(eps))
    # Lay flat exterior faces on bed (Z=0), interior features pointing upward.
    body_print = body.translate((0,depth/2,0)).rotate((0,0,0),(1,0,0),90)
    cover_print = cover.translate((0,-depth/2,0)).rotate((0,0,0),(1,0,0),-90)
    return dict(body=body,cover=cover,body_print=body_print,cover_print=cover_print,
                board=board,battery=battery,battery_max=battery_max,active=active,
                magnet=magnet_shape,rim=rim,ribs=ribs,board_pose=board_pose,screen_z=screen_z,
                usb_passage=usb_passage,board_fit=board_fit,invalid_reference_solids=invalid_reference_solids,stops=stops)


# ============================================================
# VALIDATION / EXPORT
# ============================================================
def validate(parts):
    p = dict(parts,board=parts['board_fit'])
    for name in ('body','cover','body_print','cover_print'):
        assert p[name].val().isValid(), name+' invalid'
        assert len(p[name].solids().vals()) == 1, name+' disconnected'
    pairs = [('body','board'),('body','battery_max'),('cover','board'),
             ('cover','battery_max'),('board','battery_max'),('body','magnet'),('cover','magnet'),
             ('body','usb_passage'),('cover','usb_passage')]
    for a,b in pairs:
        v = overlap(p[a],p[b])
        print(a,b,v,flush=True)
        assert v < volume_tolerance, (a,b,v)
    # Each rear stop must actually capture PCB material, not merely miss
    # components. Close the 0.15 mm gap plus 0.05 mm to test its contact footprint.
    pcb = cq.Workplane('XY').newObject([board_part(p['board'],'pcb')])
    stop_contacts = [overlap(tip.translate((0,-pcb_gap-.05,0)),pcb) for tip in p['stops']]
    assert all(v > volume_tolerance for v in stop_contacts), ('PCB stop coverage',stop_contacts)
    # Load the detached cover from ABOVE, behind the front PCB bridges.
    for travel in travel_samples:
        v = overlap(p['battery_max'].translate((0,0,travel)),p['cover'])
        assert v < volume_tolerance, ('battery top loading',travel,v)
    print('Battery top-loading path passed',flush=True)
    rigid = p['cover']
    for rib in p['ribs']:
        rigid = rigid.cut(rib)
    for travel in travel_samples:
        for label,a,b in (('cover/body',rigid.translate((0,travel,0)),p['body']),
                    ('cover/board',p['cover'].translate((0,travel,0)),p['board']),
                    ('battery/body',p['battery_max'].translate((0,travel,0)),p['body']),
                    ('board/body',p['board'].translate((0,travel,0)),p['body'])):
            print('checking',label,travel,flush=True)
            v = overlap(a,b)
            assert v < volume_tolerance, ('insertion',label,travel,v)
    rib_overlap = overlap(p['body'],p['cover'])
    assert 0 < rib_overlap < 5, ('friction ribs',rib_overlap)
    return {'intentional_rib_overlap_mm3':rib_overlap,'cad_checks':'passed','pcb_stop_contact_check_mm3':stop_contacts,
            'sampled_insertion_travel_mm':travel_samples,'battery_loading':'straight down from above into detached cover',
            'reference_solids_using_conservative_bbox':p['invalid_reference_solids']}


def main(usb='left', magnet='right'):
    p = build(usb,magnet)
    report = validate(p)
    out = OUT/f'usb-{usb}_magnet-{magnet}'
    out.mkdir(parents=True,exist_ok=True)
    for name in ('body_print','cover_print'):
        path = out/(name+'.stl')
        cq.exporters.export(p[name],str(path),tolerance=.01,angularTolerance=.1)
        mesh = trimesh.load(path,force='mesh')
        assert mesh.is_watertight and mesh.is_volume, name
        assert abs(mesh.bounds[0,2]) < eps, name+' not on bed'
    assembly = cq.Assembly()
    for name in ('body','cover','board','battery','magnet'):
        assembly.add(p[name],name=name)
    assembly.export(str(out/'assembly.step'))
    report.update(usb=usb,magnet=magnet,dimensions_mm=[width,depth,height],
                  stage='feature prototype; physical fit unverified',rim_depth_mm=rim_depth,
                  rim_clearance_mm=rim_clearance,rib_interference_mm=rim_rib_projection-rim_clearance,
                  exterior_chamfer_mm=chamfer,
                  magnet_pocket_mm=[magnet_diameter+magnet_diameter_clearance,magnet_thickness+magnet_glue_depth])
    (out/'fit-check.json').write_text(json.dumps(report,indent=2)+'\n')
    from .previews import render_magnetic_preview
    render_magnetic_preview(p,out/'preview.png',usb,magnet)
    print(out,flush=True)

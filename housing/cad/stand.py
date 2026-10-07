"""Rear-loaded enclosure with integrated bezel and unchamfered integral floor.
Feature prototype: print body bottom-down and cover outside-face-down.
"""
from pathlib import Path
import json, math
import cadquery as cq
import trimesh
from .board_reference import board_part, import_board

# PARAMETERS — mm, degrees from vertical
screen_tilt_deg=20.0
width=48.0
front_base_y=-17.5
screen_center_z=50.5  # preserve overall height with the narrower rounded crown
wall=2.4
roof_wall=1.2  # clearance for PCB and battery insertion through narrower crown
floor_t=2.4
chamfer=2.0
rear_y=28.02
cover_t=2.4
seam_gap=0.2
screen_opening_d=33.2
display_front_gap=0.15
front_lip_t=1.2
board_recess=-2.85  # corrected LCD front 1.5 -> -1.35; gap to 1.2 mm front lip
battery_dims=(34.8,10.8,52.0) # nominal width/thickness clearance, plus 2 mm pack length
battery_nominal=(34.0,10.0,50.0)
battery_tilt_deg=10.0  # top leans toward the rear cover
battery_bottom_z=4.2
battery_rear_limit_y=24.9
battery_loading_lift=3.1
battery_pocket_wall=1.2
battery_wedge_width=battery_dims[0]+2*battery_pocket_wall  # match pocket outer width
# Inner locating rim and removable cover grip.
rim_depth=3.5
rim_wall=1.4
rim_clearance=0.15
rim_rib_projection=0.20
cover_notch_width=10.0
cover_notch_depth=1.0
cover_notch_height=1.0
# Retainers stop short of PCB; optional very soft compliant pads, nominal 0.5 mm,
# rigid gap 0.15 mm fully seated / about 0.25 mm with latch play.
# Confirm pad compression physically; no rigid preload on PCB.
guide_angles=(60,90,150,210,270,300)
front_hook_angles=(240,270,300)
front_hook_overlap=0.8
front_hook_gap=0.5
front_hook_t=1.2
board_insert_tilt_deg=12.0
board_insert_raise_mm=1.2
board_insert_pivot_y=-18.25
guide_inner=18.40
recess_radius=guide_inner
base_blank_inner=18.10
guide_length=2.4
base_blank_length=guide_inner+guide_length-base_blank_inner
guide_width=3.0
guide_back=-5.15  # 0.10 mm forward of corrected PCB rear
guide_front=-front_lip_t  # bases meet the recessed display-seat plane
pad_gap=0.15
pad_r=1.0
side_pad_x=18.0
side_pad_width=1.0
side_pad_span=2.0
usb_pad_span=1.4
usb_side_pad=(19.3,-6.4)  # keep the right retainer clear of USB and curved roof
side_arm_x=19.8
arm_w=2.4
bridge_depth=2.4
pcb_back=-5.25
retainer_depth=1.6
usb_width=12.0
usb_corner_radius=0.8  # rounded USB opening corners; outer bounds stay 12 x 7 mm
usb_shell_center_z=-3.56952175415  # manufacturer USB shell bbox midpoint before board placement
usb_opening_center_z=usb_shell_center_z+board_recess
usb_rearward_mm=-6.3-usb_opening_center_z  # align original 7 mm opening to corrected PCB socket
usb_flare_depth=1.2
usb_flare_extra=0.0  # opening remains exactly 12 x 7 mm at the exterior
# PCB USB end outline from Waveshare STEP, local XY; clear by 0.15 mm.
key_clearance=0.15
key_x0=18.0
key_x1=20.7
key_wall=1.4
key_back=-2.5  # short side guides near the display face allow tilted insertion
key_front=-1.1
pcb_tab_tip_x=21.262
pcb_tab_tip_y=6.404
pcb_tab_slope=(9.187-6.404)/(21.262-15.781)
key_seat_front=-4.239  # PCB front -4.389 + 0.15 mm
key_seat_width=1.8
key_seat_y=6.4
usb_back=-9.8
usb_front=-2.8  # 7 mm depth, retaining the previous opening centre
battery_clearance=0.4
battery_rail_x=18.0
battery_rail_w=1.2
battery_rail_z=(14.0,22.0)
battery_rail_h=3.0
battery_shelf_front=12.9
battery_pocket_floor_z=2.6
battery_pocket_floor_t=1.2
battery_pocket_lip_h=3.0
battery_back_y=24.9
battery_back_pad_w=5.0
battery_back_pad_h=6.0
battery_back_pad_x=9.0
battery_back_pad_z=(14.0,40.0)
travel_samples=(0,0.5,1,2,4,6,10,15,25,40,60)
eps=0.01
OUT=Path(__file__).resolve().parents[1]/'output'/'usb-centered'


def overlap(a,b): return a.intersect(b).val().Volume()
def box(x,y,z,center): return cq.Workplane('XY').box(x,y,z).translate(center)

def build(include_board=True):
    a=math.radians(screen_tilt_deg)
    front_y=front_base_y+screen_center_z*math.tan(a)
    def pose(obj): return obj.rotate((0,0,0),(1,0,0),90-screen_tilt_deg).translate((0,front_y,screen_center_z))
    def point(x,y,z): return (x,front_y+y*math.sin(a)-z*math.cos(a),screen_center_z+y*math.cos(a)+z*math.sin(a))
    height=screen_center_z+width/2
    def arch(radius):
        return pose(cq.Workplane('XY').moveTo(-radius,-2*height).lineTo(radius,-2*height).lineTo(radius,0).threePointArc((0,radius),(-radius,0)).close().extrude(150,both=True))
    wedge=cq.Workplane('YZ').polyline([(front_base_y,0),(rear_y,0),(rear_y,height),(front_base_y+height*math.tan(a),height)]).close().extrude(width/2,both=True)
    raw=wedge.intersect(arch(width/2))
    # Exclude every edge on the build plate from the chamfer selection.
    selected=[e for e in raw.edges().vals() if e.BoundingBox().zmax>eps]
    outer=raw.newObject(selected).chamfer(chamfer)
    split_y=rear_y-cover_t
    inner_front=lambda z: front_base_y+z*math.tan(a)+wall/math.cos(a)
    cavity=cq.Workplane('YZ').polyline([(inner_front(floor_t),floor_t),(rear_y+1,floor_t),(rear_y+1,height),(inner_front(height),height)]).close().extrude(width/2-wall,both=True).intersect(arch(width/2-roof_wall))
    front_half=box(200,200,200,(0,split_y-seam_gap/2-100,80))
    rear_half=box(200,100,200,(0,split_y+seam_gap/2+50,80))
    body=outer.cut(cavity).intersect(front_half)
    cover=outer.intersect(rear_half)
    aperture=pose(cq.Workplane('XY').workplane(offset=-15).circle(screen_opening_d/2).extrude(20))
    body=body.cut(aperture)
    # Relief for the actual glass outline, leaving the integrated front lip.
    glass_relief=pose(cq.Workplane('XY').workplane(offset=-10).circle(recess_radius).extrude(10-front_lip_t))
    body=body.cut(glass_relief)
    # Shallow recess for the full display connector above the USB-side tab.
    # It shares the round display's front seating plane; no lowered shelf.
    tab_points=[(15.2,-9.6),(21.55,-6.7),(21.55,6.7),(15.2,9.6)]
    connector_relief=pose(cq.Workplane('XY').workplane(offset=-10)
                         .polyline(tab_points).close().extrude(10-front_lip_t))
    body=body.cut(connector_relief)
    for angle in guide_angles:
        guide=(cq.Workplane('XY').workplane(offset=guide_back)
               .center(base_blank_inner+base_blank_length/2,0)
               .box(base_blank_length,guide_width,guide_front-guide_back,centered=(True,True,False))
               .rotate((0,0,0),(0,0,1),angle))
        body=body.union(pose(guide))
    # Two lower rim pockets: tilt the board under the lips, then seat it.
    # Overlap is restricted to the bare PCB rim, away from header bodies.
    hook_front=pcb_back-front_hook_gap
    hook_back=hook_front-front_hook_t
    for angle in front_hook_angles:
        stem=(cq.Workplane('XY').workplane(offset=hook_back)
              .center(base_blank_inner+base_blank_length/2,0)
              .box(base_blank_length,guide_width,guide_front-hook_back,centered=(True,True,False)))
        lip=(cq.Workplane('XY').workplane(offset=hook_back)
             .center(guide_inner+(guide_length-front_hook_overlap)/2,0)
             .box(guide_length+front_hook_overlap,guide_width,front_hook_t,centered=(True,True,False)))
        hook=stem.union(lip).rotate((0,0,0),(0,0,1),angle)
        body=body.union(pose(hook))
    # Trim the bases and the display recess to the exact same cylindrical face.
    # Stop at the catching lips so their 0.8 mm inward overlap is preserved.
    base_relief=pose(cq.Workplane('XY').workplane(offset=hook_front)
                     .circle(recess_radius).extrude(-front_lip_t-hook_front+eps))
    body=body.cut(base_relief)
    usb_local=box(20,usb_width,usb_front-usb_back,
                  (27,0,(usb_front+usb_back)/2-usb_rearward_mm))
    usb_local=usb_local.edges('|X').fillet(usb_corner_radius)
    body=body.cut(pose(usb_local))
    # USB-side guide blocks removed as requested.
    # Continuous locating rim, with small side ribs supplying the press fit.
    latch_root=split_y+seam_gap/2
    section=cq.Workplane('XZ',origin=(0,latch_root,0)).add(cavity.val()).section()
    wire=section.wires().val()
    # XZ's positive normal points toward the front of the enclosure (-Y).
    outside=wire.offset2D(-rim_clearance)[0]
    inside=wire.offset2D(-(rim_clearance+rim_wall))[0]
    rim=cq.Workplane('XZ',origin=(0,latch_root,0)).add(outside).toPending().extrude(rim_depth)
    inner=cq.Workplane('XZ',origin=(0,latch_root,0)).add(inside).toPending().extrude(rim_depth+eps)
    rim=rim.cut(inner)
    # Bevel only the insertion end of the rim.
    front_edges=[e for e in rim.edges().vals() if e.BoundingBox().ymax<latch_root-rim_depth+eps]
    rim=rim.newObject(front_edges).chamfer(0.4)
    # Extend the root into the cover to guarantee a connected solid.
    root=cq.Workplane('XZ',origin=(0,latch_root,0)).add(outside).toPending().extrude(-0.4)
    root_inner=cq.Workplane('XZ',origin=(0,latch_root,0)).add(inside).toPending().extrude(-0.5)
    rim=rim.union(root.cut(root_inner))
    ribs=[]
    xface=width/2-wall-rim_clearance
    for sign in (-1,1):
        for z in (15.0,43.0):
            rib=(cq.Workplane('XY').workplane(offset=z-2.0)
                .polyline([(sign*(xface-0.1),latch_root-0.2),
                           (sign*(xface+rim_rib_projection),latch_root-0.2),
                           (sign*(xface+rim_rib_projection),latch_root-rim_depth+0.8),
                           (sign*(xface-0.1),latch_root-rim_depth+0.4)])
                .close().extrude(4.0))
            ribs.append(rib)
            rim=rim.union(rib)
    # Two clearance breaks for the battery's upper corners during loading.
    # The rest of the rim remains full thickness; every segment joins the cover.
    for sign in (-1,1):
        rim=rim.cut(box(7.0,6.0,12.0,(sign*16.0,latch_root-1.5,57.0)))
    cover=cover.union(rim)
    # Two side retainers turn outward before reaching the battery envelope.
    # Tips are parallel to the PCB, not horizontal pressure blocks.
    tip_points=[]
    for sign in (-1,1):
        x=sign*side_pad_x if sign<0 else usb_side_pad[0]
        local_y=0 if sign<0 else usb_side_pad[1]
        tip=pose(cq.Workplane('XY').workplane(offset=pcb_back-pad_gap-retainer_depth).center(x,local_y).rect(side_pad_width,side_pad_span if sign<0 else usb_pad_span).extrude(retainer_depth))
        px,py,pz=point(x,local_y,pcb_back-pad_gap-retainer_depth/2)
        shoulder_y=py+bridge_depth/2
        span=abs(side_arm_x-abs(x))+side_pad_width
        bridge=box(span,bridge_depth,arm_w,(sign*(side_arm_x+abs(x))/2,shoulder_y,pz))
        stem=box(arm_w,rear_y-shoulder_y,arm_w,(sign*side_arm_x,(rear_y+shoulder_y)/2,pz))
        cover=cover.union(bridge).union(stem)
        tip_points.append((x,local_y))
    # Upper stop remains above the maximum-size battery.
    top_x,top_y=0,16.5
    tip=pose(cq.Workplane('XY').workplane(offset=pcb_back-pad_gap-retainer_depth).center(top_x,top_y).circle(pad_r).extrude(retainer_depth))
    px,py,pz=point(top_x,top_y,pcb_back-pad_gap-retainer_depth/2)
    stem=box(arm_w,rear_y-py,arm_w,(px,(rear_y+py)/2,pz))
    cover=cover.union(stem)
    tip_points.append((top_x,top_y))
    # Battery frame: its lower end sits in an extended tilted pocket; a
    # broad wedge integral with the rear cover supports its rear face.
    ba=math.radians(battery_tilt_deg)
    by=battery_rear_limit_y-battery_dims[2]*math.sin(ba)-battery_dims[1]/2*math.cos(ba)
    bz=battery_bottom_z+battery_dims[1]/2*math.sin(ba)
    def battery_pose(obj):
        return obj.rotate((0,0,0),(1,0,0),-battery_tilt_deg).translate((0,by,bz))
    def make_pack(dims):
        return battery_pose(cq.Workplane('XY').box(*dims,centered=(True,True,False)))
    pack=make_pack(battery_dims)
    battery=make_pack(battery_nominal)
    pocket_width=battery_dims[0]+2*battery_pocket_wall
    pocket_depth=battery_dims[1]+2*battery_pocket_wall
    shelf=battery_pose(box(pocket_width,pocket_depth,battery_pocket_floor_t,
                           (0,0,-0.2-battery_pocket_floor_t/2)))
    lip=battery_pose(box(pocket_width,battery_pocket_wall,battery_pocket_lip_h+0.2,
                         (0,-battery_dims[1]/2-battery_pocket_wall/2,(battery_pocket_lip_h-0.2)/2)))
    # Back face of maximum pack: y(z) is the inclined support plane.
    back_bottom_y=by+battery_dims[1]/2*math.cos(ba)
    back_bottom_z=bz-battery_dims[1]/2*math.sin(ba)
    back_top_z=back_bottom_z+battery_dims[2]*math.cos(ba)-3.0  # roof clearance above broad support
    back_y=lambda z: back_bottom_y+(z-back_bottom_z)*math.tan(ba)
    wedge=(cq.Workplane('YZ').polyline([(back_y(battery_pocket_floor_z),battery_pocket_floor_z),
            (rear_y,battery_pocket_floor_z),(rear_y,back_top_z),(back_y(back_top_z),back_top_z)])
           .close().extrude(battery_wedge_width/2,both=True))
    cover=cover.union(wedge).union(shelf).union(lip)
    shelf_front=shelf.val().BoundingBox().ymin
    for sign in (-1,1):
        for z in battery_rail_z:
            rail=box(battery_rail_w,rear_y-shelf_front,battery_rail_h,
                     (sign*battery_rail_x,(rear_y+shelf_front)/2,z))
            cover=cover.union(rail)
    board_local=import_board().translate((0,0,board_recess)) if include_board else None
    board=pose(board_local) if include_board else None
    active=pose(cq.Workplane('XY').workplane(offset=-1.35+eps).circle(16.2).extrude(eps))
    # Shallow fingernail notch at the underside of the cover, not through it.
    notch=box(cover_notch_width,cover_notch_depth+eps,cover_notch_height+eps,
              (0,rear_y-cover_notch_depth/2+eps/2,cover_notch_height/2-eps/2))
    cover=cover.cut(notch)
    # Flat cover print surface at Z=0, retainers point up.
    cover_print=cover.translate((0,-rear_y,0)).rotate((0,0,0),(1,0,0),-90)
    return dict(body=body,cover=cover,cover_print=cover_print,board=board,board_local=board_local,
                battery=battery,battery_max=pack,active=active,pose=pose,outer=outer,
                rim=rim,ribs=ribs,battery_pose=battery_pose)


def main(check_board_insertion=True):
    p=build();OUT.mkdir(parents=True,exist_ok=True)
    report={'retention':'inner rim with battery-corner reliefs and four shallow friction ribs',
            'rim_depth_mm':3.5,'rim_wall_mm':1.4,'rim_clearance_mm':0.15,
            'rib_interference_mm':0.05,'lead_chamfer_mm':0.4,'battery_support_and_pocket_width_mm':battery_wedge_width,
            'usb_opening_mm':[12,7],'usb_rearward_mm':usb_rearward_mm,'width_mm':width,'roof_wall_mm':roof_wall,
            'usb_socket_to_outer_wall_mm':width/2-21.08,
            'physical_press_fit_test_required':True,'pcb_mm':1.6,'display_mm':2.3,'front_lip_mm':front_lip_t,'display_front_gap_mm':display_front_gap}
    outer_bounds=p['outer'].val().BoundingBox()
    report['dimensions_mm']=[round(v,3) for v in (outer_bounds.xlen,outer_bounds.ylen,outer_bounds.zlen)]
    for name in ('body','cover','cover_print'):
        assert p[name].val().isValid(), name+' invalid'
        assert len(p[name].solids().vals())==1,name+' disconnected'
    for a,b in [('body','board'),('body','battery_max'),('cover','board'),('cover','battery_max'),('board','battery_max')]:
        v=overlap(p[a],p[b]);print(a,b,v,flush=True)
        assert v<1e-5,(a,b,v)
    if check_board_insertion:
        def tilted_board(angle,raise_mm=0,travel=0):
            b=p['board_local'].rotate((0,board_insert_pivot_y,pcb_back),(1,board_insert_pivot_y,pcb_back),-angle)
            return p['pose'](b.translate((0,raise_mm,-travel)))
        for travel in travel_samples:
            assert overlap(p['body'],tilted_board(board_insert_tilt_deg,1.2,travel))<1e-5,('board approach',travel)
        for angle,shift in ((12,1.2),(10,.9),(8,.6),(6,.3),(4,.15),(2,0),(0,0)):
            v=overlap(p['body'],tilted_board(angle,shift))
            assert v<1e-5,('board lowering/pivot',angle,shift,v)
        print('Corrected PCB insertion checked',flush=True)
    rigid=p['cover']
    for rib in p['ribs']:rigid=rigid.cut(rib)
    assert overlap(rigid,p['body'])<1e-5,'Rim interference beyond friction ribs'
    report['intentional_rib_overlap_mm3']=overlap(p['cover'],p['body'])
    assert 0<report['intentional_rib_overlap_mm3']<5
    for travel in (0,0.5,1,2,3.5,5,10,20,40,60):
        moved=rigid.translate((0,travel,0))
        assert overlap(moved,p['body'])<1e-5,('cover insertion',travel)
        assert overlap(p['cover'].translate((0,travel,0)),p['board'])<1e-5
        assert overlap(p['battery_max'].translate((0,travel,0)),p['body'])<1e-5,('battery rear insertion',travel)
    local_pack=cq.Workplane('XY').box(*battery_dims,centered=(True,True,False))
    for travel in (0,2,5,10,20,40,60):
        moved=p['battery_pose'](local_pack.translate((0,-travel,battery_loading_lift)))
        assert overlap(moved,p['cover'])<1e-5,('battery loading',travel)
    for lift in (0,0.8,1.6,2.4,battery_loading_lift):
        assert overlap(p['battery_pose'](local_pack.translate((0,0,lift))),p['cover'])<1e-5
    usb=board_part(p['board_local'],'usb_shell').BoundingBox()
    lo=usb_back-usb_rearward_mm;hi=usb_front-usb_rearward_mm
    assert lo<usb.zmin<usb.zmax<hi
    assert -usb_width/2<usb.ymin<usb.ymax<usb_width/2
    path=box(30,usb.ylen,usb.zlen,(usb.xmax+15,(usb.ymin+usb.ymax)/2,(usb.zmin+usb.zmax)/2))
    blocked=overlap(p['body'],p['pose'](path))
    assert blocked<1e-5,('USB access',blocked)
    report.update(shell_access_path_collision_mm3=blocked,
                  opening_depth_interval_mm=[lo,hi],shell_depth_interval_mm=[usb.zmin,usb.zmax],
                  front_rear_shell_margin_mm=[hi-usb.zmax,usb.zmin-lo])
    for name in ('body','cover_print'):
        path=OUT/(name+'.stl');cq.exporters.export(p[name],str(path),tolerance=.01,angularTolerance=.1)
        mesh=trimesh.load(path,force='mesh');assert mesh.is_watertight and mesh.is_volume,name
    assy=cq.Assembly()
    for name in ('body','cover','board','battery'):assy.add(p[name],name=name)
    assy.export(str(OUT/'assembly.step'))
    (OUT/'fit-check.json').write_text(json.dumps(report,indent=2)+'\n')
    print(report,flush=True)
    from .previews import render_stand_views
    render_stand_views(p,OUT/'body_views.png')

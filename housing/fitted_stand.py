"""Rear-loaded enclosure with integrated bezel and unchamfered integral floor.
Feature prototype: print body bottom-down and cover outside-face-down.
"""
from pathlib import Path
import json, math
import cadquery as cq
import trimesh
from assembly_study import import_board, xy_at

# PARAMETERS — mm, degrees from vertical
screen_tilt_deg=20.0
width=52.0
front_base_y=-17.5
screen_center_z=49.5  # 2 mm taller; the battery pocket floor stays fixed
wall=2.4
floor_t=2.4
chamfer=2.0
rear_y=28.02
cover_t=2.4
seam_gap=0.2
screen_opening_d=33.2
board_recess=-2.4
battery_dims=(34.8,10.8,52.0) # nominal width/thickness clearance, plus 2 mm pack length
battery_nominal=(34.0,10.0,50.0)
battery_center=(0,19.5,29.0)
# PLA snap arms: long cantilevers, small hook engagement, side release.
snap_length=16.0
snap_t=1.2
snap_height=6.0
snap_z=28.0
snap_x=22.7
snap_hook=0.65
snap_ramp=1.4
snap_land=0.8
snap_release_ramp=1.0
snap_pocket_depth=0.8
cover_notch_width=10.0
cover_notch_depth=1.0
cover_notch_height=1.0
snap_window_gap=0.3
snap_lock_gap=0.1  # axial latch play; retain 0.3 mm on other window edges
snap_deflection=0.65
snap_root_overlap=0.3
alignment_t=2.0
alignment_depth=4.0
alignment_x=21.0
alignment_z=3.7
coupon_margin=2.0
coupon_height=10.0
# Retainers stop short of PCB; optional very soft compliant pads, nominal 0.5 mm,
# rigid gap 0.15 mm fully seated / about 0.25 mm with latch play.
# Confirm pad compression physically; no rigid preload on PCB.
guide_angles=(60,90,150,210,270,300)
front_hook_angles=(240,300)
front_hook_overlap=0.8
front_hook_gap=0.5
front_hook_t=1.2
board_insert_tilt_deg=6.0
board_insert_raise_mm=1.2
board_insert_pivot_y=-18.25
guide_inner=18.40
guide_length=2.4
guide_width=3.0
guide_back=-4.70  # low edge guides, clear of the component-side plane
guide_front=-2.3
pad_gap=0.15
pad_r=1.0
side_pad_x=18.0
side_pad_width=1.0
side_pad_span=2.0
usb_pad_span=1.4
usb_side_pad=(19.3,-6.4)  # keep the right retainer clear of USB and curved roof
side_arm_x=20.5
arm_w=2.4
bridge_depth=2.4
pcb_back=-4.8
retainer_depth=1.6
usb_width=14.0
usb_rearward_mm=3.0  # depth away from the angled front, along local -Z
usb_flare_depth=1.2
usb_flare_extra=2.4
# PCB USB end outline from Waveshare STEP, local XY; clear by 0.15 mm.
key_clearance=0.15
key_x0=18.0
key_x1=20.7
key_wall=1.4
key_back=-4.70  # stop 0.10 mm before the component-side PCB surface (-4.8)
key_front=-2.3
pcb_tab_tip_x=21.262
pcb_tab_tip_y=6.404
pcb_tab_slope=(9.187-6.404)/(21.262-15.781)
key_seat_front=-4.239  # PCB front -4.389 + 0.15 mm
key_seat_width=1.8
key_seat_y=6.4
usb_back=-12.0
usb_front=-0.6  # widen the shifted opening so its front edge clears the modeled socket
battery_clearance=0.4
battery_rail_x=18.0
battery_rail_w=1.2
battery_rail_z=(14.0,22.0)
battery_rail_h=3.0
battery_shelf_front=12.9
battery_pocket_floor_z=2.6
battery_pocket_floor_t=1.2
battery_pocket_lip_h=4.0
battery_pocket_wall=1.2
battery_back_y=24.9
battery_back_pad_w=5.0
battery_back_pad_h=6.0
battery_back_pad_x=9.0
battery_back_pad_z=(14.0,40.0)
travel_samples=(0,0.5,1,2,4,6,10,15,25,40,60)
eps=0.01
OUT=Path(__file__).parent/'output'/'taller-hidden-clips'


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
    cavity=cq.Workplane('YZ').polyline([(inner_front(floor_t),floor_t),(rear_y+1,floor_t),(rear_y+1,height),(inner_front(height),height)]).close().extrude(width/2-wall,both=True).intersect(arch(width/2-wall))
    front_half=box(200,200,200,(0,split_y-seam_gap/2-100,80))
    rear_half=box(200,100,200,(0,split_y+seam_gap/2+50,80))
    body=outer.cut(cavity).intersect(front_half)
    cover=outer.intersect(rear_half)
    aperture=pose(cq.Workplane('XY').workplane(offset=-15).circle(screen_opening_d/2).extrude(20))
    body=body.cut(aperture)
    # Relief for the actual glass outline, leaving the integrated front lip.
    glass_relief=pose(cq.Workplane('XY').workplane(offset=-10).circle(18.15).extrude(7.6))
    body=body.cut(glass_relief)
    for angle in guide_angles:
        guide=(cq.Workplane('XY').workplane(offset=guide_back)
               .center(guide_inner+guide_length/2,0)
               .box(guide_length,guide_width,guide_front-guide_back,centered=(True,True,False))
               .rotate((0,0,0),(0,0,1),angle))
        body=body.union(pose(guide))
    # Two lower rim pockets: tilt the board under the lips, then seat it.
    # Overlap is restricted to the bare PCB rim, away from header bodies.
    hook_front=pcb_back-front_hook_gap
    hook_back=hook_front-front_hook_t
    for angle in front_hook_angles:
        stem=(cq.Workplane('XY').workplane(offset=hook_back)
              .center(guide_inner+guide_length/2,0)
              .box(guide_length,guide_width,guide_front-hook_back,centered=(True,True,False)))
        lip=(cq.Workplane('XY').workplane(offset=hook_back)
             .center(guide_inner+(guide_length-front_hook_overlap)/2,0)
             .box(guide_length+front_hook_overlap,guide_width,front_hook_t,centered=(True,True,False)))
        hook=stem.union(lip).rotate((0,0,0),(0,0,1),angle)
        body=body.union(pose(hook))
    usb=pose(box(20,usb_width,usb_front-usb_back,(27,0,(usb_front+usb_back)/2)).translate((0,0,-usb_rearward_mm)))
    # A flared entry lets the cable overmould approach the socket more closely.
    flare=(cq.Workplane('YZ',origin=(width/2-usb_flare_depth,0,(usb_front+usb_back)/2))
           .rect(usb_width,usb_front-usb_back)
           .workplane(offset=usb_flare_depth+eps)
           .rect(usb_width+usb_flare_extra,usb_front-usb_back+usb_flare_extra).loft())
    body=body.cut(usb).cut(pose(flare.translate((0,0,-usb_rearward_mm))))
    # Two shaped rails key the sloping edges of the PCB's USB tab. They locate
    # the PCB outline rather than applying force to the connector shell.
    def tab_edge(x):return pcb_tab_tip_y+(pcb_tab_tip_x-x)*pcb_tab_slope
    for sign in (-1,1):
        points=[(key_x0,sign*(tab_edge(key_x0)+key_clearance)),
                (key_x1,sign*(tab_edge(key_x1)+key_clearance)),
                (key_x1,sign*(tab_edge(key_x1)+key_clearance+key_wall)),
                (key_x0,sign*(tab_edge(key_x0)+key_clearance+key_wall))]
        key=cq.Workplane('XY').workplane(offset=key_back).polyline(points).close().extrude(key_front-key_back)
        # The seat below the tab sets its depth without loading the USB shell.
        seat=box(key_x1-key_x0,key_seat_width,key_front-key_seat_front,
                 ((key_x0+key_x1)/2,sign*key_seat_y,(key_front+key_seat_front)/2))
        body=body.union(pose(key)).union(pose(seat))
    # Recessed side-release hooks; the elastic arms belong to the cover.
    latch_root=split_y+seam_gap/2
    latch_tip=latch_root-snap_length
    latches=[]
    windows=[]
    inner_side=width/2-wall
    hook_engagement=snap_x+snap_t/2+snap_hook-inner_side
    hook_end=snap_ramp+snap_land+snap_release_ramp*hook_engagement/snap_hook
    for sign in (-1,1):
        arm=box(snap_t,snap_length+snap_root_overlap,snap_height,
                (snap_x,latch_tip+(snap_length+snap_root_overlap)/2,snap_z))
        xout=snap_x+snap_t/2
        hook=(cq.Workplane('XY').workplane(offset=snap_z-snap_height/2)
              .polyline([(xout-eps,latch_tip),
                         (xout+snap_hook,latch_tip+snap_ramp),
                         (xout+snap_hook,latch_tip+snap_ramp+snap_land),
                         (xout-eps,latch_tip+snap_ramp+snap_land+snap_release_ramp)])
              .close().extrude(snap_height))
        latch=arm.union(hook)
        if sign<0: latch=latch.mirror('YZ')
        window=box(snap_pocket_depth+snap_window_gap,hook_end+snap_window_gap+snap_lock_gap,
                   snap_height+2*snap_window_gap,
                   (sign*(inner_side+(snap_pocket_depth-snap_window_gap)/2),latch_tip+(hook_end+snap_lock_gap-snap_window_gap)/2,snap_z))
        body=body.cut(window)
        cover=cover.union(latch)
        latches.append(latch);windows.append(window)
        # Short bottom guides locate the cover, with 0.3 mm floor clearance.
        guide=box(2*alignment_t,alignment_depth+snap_root_overlap,alignment_t,
                  (sign*alignment_x,latch_root-alignment_depth/2+snap_root_overlap/2,alignment_z))
        cover=cover.union(guide)
    # Small actual-size latch test: a U receiver and a matching cover strip.
    coupon_front=latch_tip-coupon_margin
    coupon_back=split_y-seam_gap/2
    receiver=box(width,coupon_margin,coupon_height,(0,coupon_front-coupon_margin/2,snap_z))
    for sign in (-1,1):
        receiver=receiver.union(box(wall,coupon_back-coupon_front,coupon_height,
                           (sign*(width/2-wall/2),(coupon_back+coupon_front)/2,snap_z)))
        receiver=receiver.cut(windows[0] if sign<0 else windows[1])
    coupon_cover=box(width,cover_t-seam_gap/2,coupon_height,
                     (0,(latch_root+rear_y)/2,snap_z))
    for latch in latches: coupon_cover=coupon_cover.union(latch)
    coupon_receiver_print=receiver.translate((0,0,-snap_z+coupon_height/2))
    coupon_cover_print=coupon_cover.translate((0,-rear_y,-snap_z)).rotate((0,0,0),(1,0,0),-90)
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
        cover=cover.union(tip).union(bridge).union(stem)
        tip_points.append((x,local_y))
    # Upper stop remains above the maximum-size battery.
    top_x,top_y=0,16.8
    tip=pose(cq.Workplane('XY').workplane(offset=pcb_back-pad_gap-retainer_depth).center(top_x,top_y).circle(pad_r).extrude(retainer_depth))
    px,py,pz=point(top_x,top_y,pcb_back-pad_gap-retainer_depth/2)
    stem=box(arm_w,rear_y-py,arm_w,(px,(rear_y+py)/2,pz))
    cover=cover.union(tip).union(stem)
    tip_points.append((top_x,top_y))
    # The pack is placed in this open cradle before installing the rear cover.
    # Side strips locate it; compliant removable tape must restrain the pouch.
    for sign in (-1,1):
        for z in battery_rail_z:
            rail=box(battery_rail_w,rear_y-battery_shelf_front,battery_rail_h,(sign*battery_rail_x,(rear_y+battery_shelf_front)/2,z))
            cover=cover.union(rail)
    # Continuous J-shaped bottom pocket: full-width shelf + front return lip.
    pocket_width=battery_nominal[0]+2*battery_clearance+2*battery_pocket_wall
    shelf=box(pocket_width,rear_y-battery_shelf_front,battery_pocket_floor_t,
              (0,(rear_y+battery_shelf_front)/2,battery_pocket_floor_z+battery_pocket_floor_t/2))
    lip=box(pocket_width,battery_pocket_wall,battery_pocket_lip_h,
            (0,battery_shelf_front+battery_pocket_wall/2,battery_pocket_floor_z+battery_pocket_floor_t+battery_pocket_lip_h/2))
    cover=cover.union(shelf).union(lip)
    for sign in (-1,1):
        for z in battery_back_pad_z:
            pad=box(battery_back_pad_w,rear_y-battery_back_y,battery_back_pad_h,
                    (sign*battery_back_pad_x,(rear_y+battery_back_y)/2,z))
            cover=cover.union(pad)
    board_local=import_board().translate((0,0,board_recess)) if include_board else None
    board=pose(board_local) if include_board else None
    pack_center=(battery_center[0],battery_center[1],battery_center[2]+(battery_dims[2]-battery_nominal[2])/2)
    pack=box(*battery_dims,pack_center)
    battery=box(*battery_nominal,battery_center)
    active=pose(cq.Workplane('XY').workplane(offset=-2.8+eps).circle(16.2).extrude(eps))
    # Shallow fingernail notch at the underside of the cover, not through it.
    notch=box(cover_notch_width,cover_notch_depth+eps,cover_notch_height+eps,
              (0,rear_y-cover_notch_depth/2+eps/2,cover_notch_height/2-eps/2))
    cover=cover.cut(notch)
    # Flat cover print surface at Z=0, retainers point up.
    cover_print=cover.translate((0,-rear_y,0)).rotate((0,0,0),(1,0,0),-90)
    return dict(body=body,cover=cover,cover_print=cover_print,board=board,board_local=board_local,battery=battery,battery_max=pack,active=active,pose=pose,outer=outer,tip_points=tip_points,latches=latches,
                coupon_receiver_print=coupon_receiver_print,coupon_cover_print=coupon_cover_print)


def verify(p):
    report={'stage':'Physical-fit revision: rearward USB, low PCB stops and battery pocket','screen_angle_deg':screen_tilt_deg,'floor_mm':floor_t,'bottom_chamfer_mm':0,'retention':'Integrated front lip; three padded retainers on rear cover; two concealed pull-release PLA snap arms'}
    assert abs(p['cover_print'].val().BoundingBox().zmin)<1e-5, 'Cover print face not on bed'
    for key in ('body','cover','cover_print','coupon_receiver_print','coupon_cover_print'):
        assert p[key].val().isValid(),key+' invalid'
        assert len(p[key].solids().vals())==1,key+' disconnected'
    for aa,bb in (('body','board'),('body','battery_max'),('cover','board'),('cover','battery_max'),('body','cover')):
        v=overlap(p[aa],p[bb]); print(aa,bb,v,flush=True)
        if v>=1e-5:
            for solid in p[aa].intersect(p[bb]).solids().vals():
                b=solid.BoundingBox(); print('collision bbox',b.xmin,b.xmax,b.ymin,b.ymax,b.zmin,b.zmax,flush=True)
        assert v<1e-5,f'{aa}/{bb}: {v}'
    # Simple cantilever strain estimate, not a fatigue or FEA validation.
    report['snap']={'length_mm':snap_length,'thickness_mm':snap_t,
                    'required_tip_deflection_mm':snap_deflection,
                    'estimated_root_strain_percent':round(100*1.5*snap_t*snap_deflection/snap_length**2,3),
                    'radial_hook_engagement_mm':round(snap_x+snap_t/2+snap_hook-(width/2-wall),3),
                    'axial_latch_play_mm':snap_lock_gap,
                    'pad_thickness_nominal_mm':0.5,
                    'outside_wall_remaining_mm':wall-snap_pocket_depth,
                    'release':'pull cover using underside notch; rear hook ramps flex the arms inward',
                    'physical_fit_test_required':True}
    cover_rigid=p['cover']
    for latch in p['latches']: cover_rigid=cover_rigid.cut(latch)
    # Conservative clearance proxy: translate each whole arm inward. This is
    # not a deformation simulation; analytical strain and physical coupon
    # testing address the elastic motion separately.
    flex_proxy=cover_rigid
    for sign,latch in zip((-1,1),p['latches']):
        flex_proxy=flex_proxy.union(latch.translate((-sign*snap_deflection,0,0)))
    assert overlap(flex_proxy,p['board'])<1e-5
    assert overlap(flex_proxy,p['battery_max'])<1e-5
    lock_overlap=overlap(p['cover'].translate((0,0.5,0)),p['body'])
    assert lock_overlap>0.1, 'Hooks fail to block withdrawal'
    report['snap']['unreleased_withdrawal_overlap_mm3']=round(lock_overlap,4)
    report['rear_insertion_overlap_mm3']={}
    # Display first, battery second, cover last. All move through the open rear.
    for name in ('battery_max','cover'):
        checks={}
        for travel in travel_samples:
            part=flex_proxy if name=='cover' and travel>0 else p[name]
            moved=p['pose'](p['board_local'].translate((0,0,-travel))) if name=='board' else part.translate((0,travel,0))
            v=overlap(moved,p['body'])
            if name in ('battery_max','cover'): v+=overlap(moved,p['board'])
            # Cover and its temporarily secured battery move together.
            if name=='cover': v+=overlap(moved,p['battery_max'].translate((0,travel,0)))
            assert v<1e-5,f'{name} rear insertion {travel}: {v}'
            checks[str(travel)]=round(v,8)
        report['rear_insertion_overlap_mm3'][name]=checks
        print(name+' insertion checked',flush=True)
    report['board_hook_insertion_overlap_mm3']={}
    def tilted_board(angle,raise_mm=0,travel=0):
        b=p['board_local'].rotate((0,board_insert_pivot_y,pcb_back),(1,board_insert_pivot_y,pcb_back),-angle)
        return p['pose'](b.translate((0,raise_mm,-travel)))
    for travel in travel_samples:
        v=overlap(p['body'],tilted_board(board_insert_tilt_deg,board_insert_raise_mm,travel))
        assert v<1e-5,f'Board angled approach {travel}: {v}'
        report['board_hook_insertion_overlap_mm3'][f'approach_{travel}']=round(v,8)
    for shift in (board_insert_raise_mm,0.9,0.6,0.3,0):
        v=overlap(p['body'],tilted_board(board_insert_tilt_deg,shift))
        assert v<1e-5,f'Board slide under hooks {shift}: {v}'
        report['board_hook_insertion_overlap_mm3'][f'slide_{shift}']=round(v,8)
    for angle in (6,4,2,0):
        v=overlap(p['body'],tilted_board(angle))
        assert v<1e-5,f'Board pivot into seat {angle}: {v}'
        report['board_hook_insertion_overlap_mm3'][f'pivot_{angle}']=round(v,8)
    print('Tilt-and-slide board insertion checked',flush=True)
    b=p['outer'].val().BoundingBox()
    report['dimensions_mm']=[round(b.xlen,2),round(b.ylen,2),round(b.zlen,2)]
    report['battery_pocket_loading_overlap_mm3']={}
    # Battery enters the cover from the front, raised 5 mm above its final
    # position to clear the pocket lip, then drops into the pocket. Revised
    # side retainers lie outside this entire insertion envelope.
    for travel in (0,2,5,10,20,40,60):
        moved=p['battery_max'].translate((0,-travel,5))
        v=overlap(moved,p['cover']);assert v<1e-5,f'Battery pocket front insertion {travel}: {v}'
        report['battery_pocket_loading_overlap_mm3'][f'front_{travel}']=round(v,8)
    for lift in (0,1,2,3,4,5):
        v=overlap(p['battery_max'].translate((0,0,lift)),p['cover'])
        assert v<1e-5,f'Battery pocket lowering {lift}: {v}'
        report['battery_pocket_loading_overlap_mm3'][f'lower_{lift}']=round(v,8)
    report['fit_changes']={'usb_rearward_mm':usb_rearward_mm,'usb_shift_axis':'away from angled front, local -Z','body_width_mm':width,
        'usb_face_setback_mm':round(width/2-21.08,2),'usb_flare_depth_mm':usb_flare_depth,
        'guide_nominal_radial_gap_mm':round(guide_inner-18.25,2),
        'key_edge_gap_mm':key_clearance,'rigid_pcb_pad_gap_mm':pad_gap,
        'battery_nominal_mm':battery_nominal,'battery_pocket_inside_mm':[34.8,10.8],
        'battery_pocket_lip_height_mm':battery_pocket_lip_h,
        'extra_housing_height_mm':2.0,'battery_length_allowance_mm':battery_dims[2],
        'front_hook_overlap_mm':front_hook_overlap,'front_hook_pcb_gap_mm':front_hook_gap,
        'rotation_stop_back_z_mm':key_back}
    report['pcb_rotation_blocked_at_degrees']={}
    pcb=p['board_local'].solids().vals()[0]
    for angle in (-1,1):
        moved=p['pose'](cq.Workplane('XY').newObject([pcb]).rotate((0,0,0),(0,0,1),angle))
        v=overlap(p['body'],moved)
        assert v>1e-4, 'PCB rotation stop missing'
        report['pcb_rotation_blocked_at_degrees'][str(angle)]=round(v,6)
    report['assembly_order']='Tilt board 6 degrees rearward, slide under lower rim pockets, pivot into seat; slide battery into cover 5 mm raised, lower into pocket; install cover and battery together.' 
    report['pending']=['physical fit of pads, snap coupon and cable','battery padding and wire routing','coffee-machine surface temperature: PLA prototype, no heat suitability confirmed','slicer supports inside roof and display opening']
    return report


def main():
    p=build(); report=verify(p); OUT.mkdir(parents=True,exist_ok=True)
    report['meshes']={}
    for name in ('body','cover_print','coupon_receiver_print','coupon_cover_print'):
        path=OUT/(name+'.stl');cq.exporters.export(p[name],str(path),tolerance=.01,angularTolerance=.1)
        mesh=trimesh.load(path,force='mesh');assert mesh.is_watertight and mesh.is_volume,name
        report['meshes'][name]={'watertight':True,'volume_mm3':round(mesh.volume,2)}
    assy=cq.Assembly()
    for name in ('body','cover','board','battery'):assy.add(p[name],name=name)
    assy.export(str(OUT/'assembly.step'))
    (OUT/'fit-check.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report),flush=True)
    from render_fitted import main as render
    render(p,report)

if __name__=='__main__':main()

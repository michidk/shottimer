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
width=54.0
front_base_y=-17.5
screen_center_z=47.5
wall=2.4
floor_t=2.4
chamfer=2.0
rear_y=28.02
cover_t=2.4
seam_gap=0.2
screen_opening_d=33.2
board_recess=-2.4
battery_dims=(36.0,12.0,52.0) # maximum, includes maker's +/-2 mm
battery_nominal=(34.0,10.0,50.0)
battery_center=(0,19.5,29.0)
# PLA snap arms: long cantilevers, small hook engagement, side release.
snap_length=16.0
snap_t=1.2
snap_height=6.0
snap_z=28.0
snap_x=23.7
snap_hook=0.65
snap_ramp=1.4
snap_land=0.8
snap_window_gap=0.3
snap_deflection=0.65
snap_root_overlap=0.3
alignment_t=2.0
alignment_depth=4.0
alignment_x=22.0
alignment_z=3.7
coupon_margin=2.0
coupon_height=10.0
# Retainers stop short of PCB; add very soft compliant pads, nominal 1.0 mm,
# thickness 0.4 mm fully seated / up to 0.7 mm with latch play.
# Confirm pad compression physically; no rigid preload on PCB.
guide_angles=(90,210,300)
guide_inner=18.65
guide_length=2.4
guide_width=3.0
guide_back=-5.8
guide_front=-2.3
pad_gap=0.4
pad_r=1.0
side_pad_x=17.0
usb_side_pad=(15.0,-9.0)  # keep the right retainer clear of USB and curved roof
side_arm_x=20.5
arm_w=2.4
bridge_depth=2.4
pcb_back=-4.8
retainer_depth=1.6
usb_width=14.0
usb_back=-10.0
usb_front=-2.6
battery_rail_x=18.9
battery_rail_w=1.2
battery_rail_z=(14.0,22.0)
battery_rail_h=3.0
battery_shelf_front=13.0
travel_samples=(0,0.5,1,2,4,6,10,15,25,40,60)
eps=0.01
OUT=Path(__file__).parent/'output'/'screwless'


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
    usb=pose(box(20,usb_width,usb_front-usb_back,(27,0,(usb_front+usb_back)/2)))
    body=body.cut(usb)
    # Recessed side-release hooks; the elastic arms belong to the cover.
    latch_root=split_y+seam_gap/2
    latch_tip=latch_root-snap_length
    latches=[]
    windows=[]
    for sign in (-1,1):
        arm=box(snap_t,snap_length+snap_root_overlap,snap_height,
                (snap_x,latch_tip+(snap_length+snap_root_overlap)/2,snap_z))
        xout=snap_x+snap_t/2
        hook=(cq.Workplane('XY').workplane(offset=snap_z-snap_height/2)
              .polyline([(xout-eps,latch_tip),
                         (xout+snap_hook,latch_tip+snap_ramp),
                         (xout+snap_hook,latch_tip+snap_ramp+snap_land),
                         (xout-eps,latch_tip+snap_ramp+snap_land)])
              .close().extrude(snap_height))
        latch=arm.union(hook)
        if sign<0: latch=latch.mirror('YZ')
        window=box(wall+snap_hook+2,snap_ramp+snap_land+2*snap_window_gap,
                   snap_height+2*snap_window_gap,
                   (sign*(width/2-wall/2),latch_tip+(snap_ramp+snap_land)/2,snap_z))
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
        tip=pose(cq.Workplane('XY').workplane(offset=pcb_back-pad_gap-retainer_depth).center(x,local_y).circle(pad_r).extrude(retainer_depth))
        px,py,pz=point(x,local_y,pcb_back-pad_gap-retainer_depth/2)
        shoulder_y=py+bridge_depth/2
        span=abs(side_arm_x-abs(x))+arm_w
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
    # No shelf under the pouch: the integral floor supports it with a soft pad.
    board_local=import_board().translate((0,0,board_recess)) if include_board else None
    board=pose(board_local) if include_board else None
    pack=box(*battery_dims,battery_center)
    battery=box(*battery_nominal,battery_center)
    active=pose(cq.Workplane('XY').workplane(offset=-2.8+eps).circle(16.2).extrude(eps))
    # Flat cover print surface at Z=0, retainers point up.
    cover_print=cover.translate((0,-rear_y,0)).rotate((0,0,0),(1,0,0),-90)
    return dict(body=body,cover=cover,cover_print=cover_print,board=board,board_local=board_local,battery=battery,battery_max=pack,active=active,pose=pose,outer=outer,tip_points=tip_points,latches=latches,
                coupon_receiver_print=coupon_receiver_print,coupon_cover_print=coupon_cover_print)


def verify(p):
    report={'stage':'Screwless rear-access feature prototype','screen_angle_deg':screen_tilt_deg,'floor_mm':floor_t,'bottom_chamfer_mm':0,'retention':'Integrated front lip; three padded retainers on rear cover; two side-release PLA snap arms'}
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
                    'axial_latch_play_mm':snap_window_gap,
                    'pad_thickness_nominal_mm':1.0,
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
    for name in ('board','battery_max','cover'):
        checks={}
        for travel in travel_samples:
            part=flex_proxy if name=='cover' and travel>0 else p[name]
            moved=part.translate((0,travel,0))
            v=overlap(moved,p['body'])
            if name in ('battery_max','cover'): v+=overlap(moved,p['board'])
            # Cover and its temporarily secured battery move together.
            if name=='cover': v+=overlap(moved,p['battery_max'].translate((0,travel,0)))
            assert v<1e-5,f'{name} rear insertion {travel}: {v}'
            checks[str(travel)]=round(v,8)
        report['rear_insertion_overlap_mm3'][name]=checks
        print(name+' insertion checked',flush=True)
    b=p['outer'].val().BoundingBox()
    report['dimensions_mm']=[round(b.xlen,2),round(b.ylen,2),round(b.zlen,2)]
    report['assembly_order']='Display first; secure padded battery to rear-cover cradle; insert cover and battery together.'
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
    from render_screwless import main as render
    render(p,report)

if __name__=='__main__':main()

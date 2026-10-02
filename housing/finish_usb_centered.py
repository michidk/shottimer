import os,json,runpy,sys
os.environ['PYOPENGL_PLATFORM']='egl'
import numpy as np
np.infty=np.inf
import cadquery as cq,trimesh
import usb_centered_stand as m
import curved_holder_stand as old
p=m.build();before=old.build(False)
m.OUT.mkdir(parents=True,exist_ok=True)
added=p['body'].cut(before['body'])
for n in ('board','battery_max','cover'):
 v=m.overlap(added,p[n]);assert v<1e-5,(n,v)
print('Added wall material clears assembled components',flush=True)
for travel in m.travel_samples:
 b=p['board_local'].rotate((0,m.board_insert_pivot_y,m.pcb_back),(1,m.board_insert_pivot_y,m.pcb_back),-12).translate((0,1.2,-travel))
 assert m.overlap(added,p['pose'](b))<1e-5,('approach',travel)
for angle,shift in ((12,1.2),(10,.9),(8,.6),(6,.3),(4,.15),(2,0),(0,0)):
 b=p['board_local'].rotate((0,m.board_insert_pivot_y,m.pcb_back),(1,m.board_insert_pivot_y,m.pcb_back),-angle).translate((0,shift,0))
 assert m.overlap(added,p['pose'](b))<1e-5,('seating',angle)
for travel in (0,.5,1,2,3.5,5,10,20,40,60):
 for name in ('cover','battery_max'):
  assert m.overlap(added,p[name].translate((0,travel,0)))<1e-5,(name,travel)
usb=p['board_local'].solids().vals()[80].BoundingBox()
lo=m.usb_back-m.usb_rearward_mm;hi=m.usb_front-m.usb_rearward_mm
assert lo<usb.zmin<usb.zmax<hi
assert -m.usb_width/2<usb.ymin<usb.ymax<m.usb_width/2
# Extend the complete metal shell silhouette out through the side wall.
path=m.box(30,usb.ylen,usb.zlen,(usb.xmax+15,(usb.ymin+usb.ymax)/2,(usb.zmin+usb.zmax)/2))
blocked=m.overlap(p['body'],p['pose'](path));assert blocked<1e-5,blocked
for name in ('body','cover_print'):
 assert p[name].val().isValid() and len(p[name].solids().vals())==1
 dest=m.OUT/(name+'.stl');cq.exporters.export(p[name],str(dest),tolerance=.01,angularTolerance=.1)
 mesh=trimesh.load(dest,force='mesh');assert mesh.is_watertight and mesh.is_volume
assy=cq.Assembly()
for name in ('body','cover','board','battery'):assy.add(p[name],name=name)
assy.export(str(m.OUT/'assembly.step'))
report={'opening_mm':[12,7],'corner_radius_mm':m.usb_corner_radius,'moved_toward_display_mm':old.usb_rearward_mm-m.usb_rearward_mm,'opening_depth_interval_mm':[lo,hi],'shell_depth_interval_mm':[usb.zmin,usb.zmax],'front_rear_shell_margin_mm':[hi-usb.zmax,usb.zmin-lo],'shell_access_path_collision_mm3':blocked,'validation':'Single valid watertight printable solids; added wall checked against seated parts and sampled PCB, cover and battery insertion. Full shell silhouette clears wall. Cable overmould not modeled.'}
(m.OUT/'fit-check.json').write_text(json.dumps(report,indent=2)+'\n')
print(report,flush=True)
runpy.run_path('render_usb_centered.py',init_globals={'parts':p})
from render_housing_views import render_views
render_views(p,m.OUT/'body_views.png')

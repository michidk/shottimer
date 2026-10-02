import json,math
import cadquery as cq,trimesh
import curved_holder_stand as m
import aligned_holders_stand as old
p=m.build();before=old.build(False)
added=p['body'].cut(before['body'])
report={'recess_and_base_radius_mm':m.recess_radius,'radial_lip_overlap_mm':m.front_hook_overlap,'pcb_rear_gap_mm':m.front_hook_gap,'change':'Holder inner faces and recess share one cylinder. Only added material needs new collision checks; all other geometry was checked in aligned-holders.'}
for n in ('board','battery_max','cover'):
 v=m.overlap(added,p[n]);assert v<1e-5,(n,v)
for travel in m.travel_samples:
 b=p['board_local'].rotate((0,m.board_insert_pivot_y,m.pcb_back),(1,m.board_insert_pivot_y,m.pcb_back),-12).translate((0,1.2,-travel))
 assert m.overlap(added,p['pose'](b))<1e-5,('approach',travel)
for angle,shift in ((12,1.2),(10,.9),(8,.6),(6,.3),(4,.15),(2,0),(0,0)):
 b=p['board_local'].rotate((0,m.board_insert_pivot_y,m.pcb_back),(1,m.board_insert_pivot_y,m.pcb_back),-angle).translate((0,shift,0))
 assert m.overlap(added,p['pose'](b))<1e-5,('seating',angle)
m.OUT.mkdir(parents=True,exist_ok=True)
for n in ('body','cover_print'):
 assert p[n].val().isValid() and len(p[n].solids().vals())==1
 path=m.OUT/(n+'.stl');cq.exporters.export(p[n],str(path),tolerance=.01,angularTolerance=.1)
 mesh=trimesh.load(path,force='mesh');assert mesh.is_watertight and mesh.is_volume
assy=cq.Assembly()
for n in ('body','cover','board','battery'):assy.add(p[n],name=n)
assy.export(str(m.OUT/'assembly.step'))
(m.OUT/'fit-check.json').write_text(json.dumps(report,indent=2)+'\n')
print('Curved bases validated',flush=True)
from render_curved_holders import main
main(p)

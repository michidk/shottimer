from assembly_study import import_board
from pathlib import Path
import numpy as np,json
s=import_board(False,False).solids().vals();pcb=s[0]
points=np.array([v.toTuple()[:2] for v in pcb.Vertices() if v.toTuple()[2]>-2 and v.toTuple()[0]<13 and np.linalg.norm(v.toTuple()[:2])>17.5])
x,y=points.T
cx,cy,c=np.linalg.lstsq(np.column_stack((2*x,2*y,np.ones(len(x)))),x*x+y*y,rcond=None)[0]
r=float(np.sqrt(c+cx*cx+cy*cy))
midr=[]
for e in pcb.Edges():
 b=e.BoundingBox()
 if b.zmin>-2 and b.xmax<13 and e.geomType()=='LINE' and 1.30<e.Length()<1.31:
  p=e.positionAt(.5);midr.append(float(np.hypot(p.x-cx,p.y-cy)))
h=[s[i].BoundingBox() for i in (46,47)]
report={'radius_at_facets_mm':float(np.mean(midr)), 'radius_at_vertices_mm':r,
 'tab_shoulder_width_mm':9.187276-(-9.185075),
 'tab_end_width_mm':6.403614-(-6.403252),
 'tab_length_sides_mm':[21.262054-15.780775,21.262054-15.782782],
 'header_center_spacing_mm':(h[0].ymin+h[0].ymax-h[1].ymin-h[1].ymax)/2,
 'header_pitch':'1.27 mm specified by supplied drawing; cannot verify in simplified header blocks',
 'header_volumes_mm3':[s[i].Volume() for i in (46,47)],
 'header_bbox_volumes_mm3':[b.xlen*b.ylen*b.zlen for b in h],
 'measured_stack_mm':{'pcb':1.6,'display':2.3,'total':3.9}}
Path('output/dimension-audit/pcb-dimensions.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))

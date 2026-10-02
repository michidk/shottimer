"""Populated manufacturer board and proposed contact-point audit."""
import os
os.environ['PYOPENGL_PLATFORM']='egl'
import json
from pathlib import Path
import numpy as np
np.infty=np.inf
import cadquery as cq
import pyrender
from PIL import Image,ImageDraw,ImageFont
from assembly_study import import_board
from render_concept import mesh,camera_pose,FONT,BOLD
from fitted_stand import OUT,side_pad_x,usb_side_pad,side_pad_width,side_pad_span,usb_pad_span,pad_r

OUT.mkdir(parents=True,exist_ok=True)
b=import_board().translate((0,0,-2.4));solids=b.solids().vals()
scene=pyrender.Scene(bg_color=[.95,.96,.97,1],ambient_light=[.6,.6,.6])
groups={}
for i,s in enumerate(solids):
 c=(.05,.32,.65,1) if i==0 else ((.045,.045,.055,1) if i in (46,47,56,116) else (.67,.69,.69,1))
 groups.setdefault(c,[]).append(s)
for c,ss in groups.items():
 shape=cq.Workplane('XY').newObject([cq.Compound.makeCompound(ss)]).rotate((0,0,0),(1,0,0),180)
 scene.add(pyrender.Mesh.from_trimesh(mesh(shape),material=pyrender.MetallicRoughnessMaterial(baseColorFactor=c,roughnessFactor=.6),smooth=False))
cam=camera_pose(-90,89.9,(0,0,0))
scene.add(pyrender.OrthographicCamera(xmag=24,ymag=24,zfar=1000),pose=cam)
for az,el,power in ((-70,60,2.3),(130,40,1.0)):
 scene.add(pyrender.DirectionalLight(color=np.ones(3),intensity=power),pose=camera_pose(az,el,(0,0,0)))
renderer=pyrender.OffscreenRenderer(1100,1100)
color,_=renderer.render(scene);renderer.delete()
im=Image.fromarray(color);d=ImageDraw.Draw(im)
points=[('A: left edge',-side_pad_x,0,False),('B: USB tab',*usb_side_pad,False),('C: top edge',0,16.8,True)]
components=cq.Workplane('XY').newObject([cq.Compound.makeCompound(solids[1:])])
report={}
for label,x,y,circular in points:
 footprint=cq.Workplane('XY').workplane(offset=-20).center(x,y)
 footprint=footprint.circle(pad_r) if circular else footprint.rect(side_pad_width,usb_pad_span if x>0 else side_pad_span)
 probe=footprint.extrude(15.2-0.01)
 v=probe.intersect(components).val().Volume()
 report[label]={'xy_mm':[x,y],'component_volume_in_rear_contact_column_mm3':v}
 assert v<1e-5,(label,v)
 # Orthographic near-top view: board is rotated X180; original Y maps downward.
 px=550+x*1100/48;py=550+y*1100/48
 d.ellipse((px-9,py-9,px+9,py+9),fill='#f47725',outline='white',width=2)
 tx=50 if x<0 else (740 if x>0 else 570)
 ty=py-50
 d.line((px,py,tx+10,ty+25),fill='#c24d08',width=3)
 d.text((tx,ty),label,font=ImageFont.truetype(BOLD,22),fill='#c24d08')
d.text((25,20),'Manufacturer STEP: populated PCB + revised finger contacts',font=ImageFont.truetype(BOLD,25),fill='#253038')
d.text((25,1060),'Orange = intended PCB contact. Model accuracy must be compared with the physical board.',font=ImageFont.truetype(FONT,20),fill='#253038')
im.save(OUT/'board_contacts.png')
(OUT/'contact-audit.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report),flush=True)

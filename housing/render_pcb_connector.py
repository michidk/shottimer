"""Display-side render of the manufacturer module, without enclosure."""
import os
os.environ['PYOPENGL_PLATFORM']='egl'
import numpy as np
np.infty=np.inf
import cadquery as cq,pyrender
from PIL import Image,ImageDraw,ImageFont
from assembly_study import import_board
from render_concept import mesh,camera_pose,FONT,BOLD
from pathlib import Path
out=Path('output/press-fit-rim');out.mkdir(parents=True,exist_ok=True)
board=import_board()
sheet=Image.new('RGB',(1500,850),'#f0f2f4')
for index,(az,el) in enumerate(((-55,62),(-90,90))):
 scene=pyrender.Scene(bg_color=[.94,.95,.96,1],ambient_light=[.55,.55,.55])
 for i,solid in enumerate(board.solids().vals()):
  color=([.95,.40,.06,1] if i==len(board.solids().vals())-1 else ([.08,.37,.28,1] if i==0 else ([.035,.06,.075,1] if i==116 else [.55,.59,.63,1])))
  material=pyrender.MetallicRoughnessMaterial(baseColorFactor=color,roughnessFactor=.6,doubleSided=True)
  scene.add(pyrender.Mesh.from_trimesh(mesh(cq.Workplane('XY').newObject([solid])),material=material,smooth=False))
 scene.add(pyrender.OrthographicCamera(xmag=24,ymag=24,zfar=1000),pose=camera_pose(az,el,(0,0,0)))
 for a,e,p in [(-50,65,2),(100,40,1)]:
  scene.add(pyrender.DirectionalLight(color=np.ones(3),intensity=p),pose=camera_pose(a,e,(0,0,0)))
 r=pyrender.OffscreenRenderer(750,750)
 pixels,_=r.render(scene);r.delete()
 sheet.paste(Image.fromarray(pixels),(index*750,70))
d=ImageDraw.Draw(sheet)
d.text((35,20),'Display side — connector extension highlighted',font=ImageFont.truetype(BOLD,30),fill='#253038')
d.text((35,809),'Orange: added connector clearance envelope, reaching the same front surface as the display.',font=ImageFont.truetype(FONT,22),fill='#253038')
sheet.save(out/'pcb_connector_highlighted.png')
print(out/'pcb_connector_highlighted.png')

"""Rear view of front housing with populated PCB installed."""
from pathlib import Path
from PIL import Image,ImageDraw,ImageFont
from render_compact import FONT,BOLD
from render_concept import mesh,camera_pose
import pyrender,numpy as np,cadquery as cq

def render(parts,names,size,azimuth,elevation,center,scale):
    scene=pyrender.Scene(bg_color=[.94,.95,.96,1],ambient_light=[.65,.65,.65])
    shapes=[(parts['body'],[.52,.56,.58,1])]
    for i,solid in enumerate(parts['board'].solids().vals()):
        color=[.07,.38,.27,1] if i==0 else [.58,.61,.64,1]
        shapes.append((cq.Workplane('XY').newObject([solid]),color))
    for shape,color in shapes:
        mat=pyrender.MetallicRoughnessMaterial(baseColorFactor=color,roughnessFactor=.8,doubleSided=True)
        scene.add(pyrender.Mesh.from_trimesh(mesh(shape),material=mat,smooth=False))
    w,h=size
    scene.add(pyrender.OrthographicCamera(xmag=scale*w/h,ymag=scale,zfar=1000),pose=camera_pose(azimuth,elevation,center))
    for az,el,power in [(80,25,2.2),(140,40,1.1),(-30,60,.7)]:
        scene.add(pyrender.DirectionalLight(color=np.ones(3),intensity=power),pose=camera_pose(az,el,center))
    renderer=pyrender.OffscreenRenderer(w,h)
    pixels,_=renderer.render(scene)
    renderer.delete()
    return Image.fromarray(pixels)
from press_fit_stand import build,OUT
p=build()
OUT.mkdir(exist_ok=True,parents=True)
sheet=Image.new('RGB',(1600,950),'#f0f2f4')
for i,(az,el) in enumerate(((80,22),(90,0))):
    view=render(p,['body','board'],(800,800),azimuth=az,elevation=el,center=(0,5,38),scale=42)
    sheet.paste(view,(800*i,80))
d=ImageDraw.Draw(sheet)
d.text((35,22),'Front housing with the populated PCB installed',font=ImageFont.truetype(BOLD,32),fill='#253038')
d.text((35,902),'Rear cover and battery removed for visibility. Green: PCB; grey: components from the manufacturer CAD model.',font=ImageFont.truetype(FONT,23),fill='#253038')
sheet.save(OUT/'front_with_pcb.png')
print(OUT/'front_with_pcb.png')

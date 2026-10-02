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
from aligned_holders_stand import OUT

def main(p):
    OUT.mkdir(exist_ok=True,parents=True)
    sheet=Image.new('RGB',(1600,950),'#f0f2f4')
    for i,(az,el) in enumerate(((80,22),(90,0))):
        view=render(p,['body','board'],(800,800),azimuth=az,elevation=el,center=(0,5,38),scale=42)
        sheet.paste(view,(800*i,80))
    d=ImageDraw.Draw(sheet)
    d.text((35,22),'Aligned holder bases; USB-side blocks removed',font=ImageFont.truetype(BOLD,32),fill='#253038')
    d.text((35,900),'Holder bases meet the display recess. PCB-catching lips retain 0.5 mm rear clearance.',font=ImageFont.truetype(FONT,23),fill='#253038')
    sheet.save(OUT/'front_with_pcb.png')
    from render_compact import render as render_plain
    from compact_usb_stand import build as old_build
    old=old_build(False)
    compare=Image.new('RGB',(1600,880),'#f0f2f4')
    for i,(part,label) in enumerate(((old,'BEFORE: stepped bases and USB guides'),(p,'AFTER: aligned bases; guides removed'))):
        picture=render_plain(part,['body'],(800,780),azimuth=65,elevation=25,center=(7,5,49),scale=28)
        compare.paste(picture,(800*i,80))
        ImageDraw.Draw(compare).text((800*i+25,25),label,font=ImageFont.truetype(BOLD,25),fill='#253038')
    compare.save(OUT/'usb_holders_before_after.png')
    covers=Image.new('RGB',(1600,900),'#f0f2f4')
    for i,(part,label) in enumerate(((old,'BEFORE: raised contact pads'),(p,'AFTER: flat-ended rear fingers'))):
        picture=render_plain(dict(body=part['cover']),['body'],(800,800),azimuth=-65,elevation=23,center=(0,20,39),scale=37)
        covers.paste(picture,(800*i,80))
        ImageDraw.Draw(covers).text((800*i+25,25),label,font=ImageFont.truetype(BOLD,25),fill='#253038')
    covers.save(OUT/'rear_fingers_before_after.png')
    import sys
    sys.path.insert(0,'/home/michi/.codex/skills/parametric-3d-printing')
    import preview
    for n in ('body','cover_print'):
        preview.render_multi_view(preview.load_mesh(str(OUT/(n+'.stl'))),str(OUT/(n+'_views.png')),title=n.replace('_',' ').title())

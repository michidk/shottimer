"""Render the revised compact model and a true geometric cutaway."""
import os
os.environ.setdefault('PYOPENGL_PLATFORM','egl')
import json
import math
import numpy as np
np.infty=np.inf  # pyrender 0.1.45 compatibility
import cadquery as cq
import trimesh
import pyrender
from PIL import Image, ImageDraw, ImageFont
from compact_stand import build, OUT
from render_concept import mesh, camera_pose, FONT, BOLD


def render(parts,names,size,azimuth=-55,elevation=23,center=(0,4,33),scale=46,section=False,exploded=False,ground=True):
    scene=pyrender.Scene(bg_color=[.94,.95,.96,1],ambient_light=[.5,.5,.5])
    colors={'body':[.28,.34,.35,1], 'bezel':[.25,.31,.32,1], 'lid':[.23,.28,.30,1],
            'board':[.12,.35,.30,1], 'active':[.035,.20,.23,1], 'battery':[.23,.51,.77,1],
            'gasket':[.07,.09,.1,1]}
    cutter=cq.Workplane('XY').box(100,300,250).translate((50,0,80))
    for name in names:
        shape=parts[name]
        if exploded:
            if name=='board': shape=parts['pose'](parts['board_local'].translate((0,0,24)))
            if name=='bezel': shape=parts['pose'](parts['bezel_local'].translate((0,0,48)))
        if section:
            shape=shape.cut(cutter)
        mat=pyrender.MetallicRoughnessMaterial(baseColorFactor=colors[name],roughnessFactor=.68,
                                              metallicFactor=.08,doubleSided=True)
        scene.add(pyrender.Mesh.from_trimesh(mesh(shape),material=mat,smooth=False))
    if ground:
        floor=trimesh.creation.box(extents=[230,230,.1])
        floor.apply_translation([0,0,-.12])
        scene.add(pyrender.Mesh.from_trimesh(floor,material=pyrender.MetallicRoughnessMaterial(
            baseColorFactor=[.91,.92,.93,1],roughnessFactor=1)))
    w,h=size
    pose=camera_pose(azimuth,elevation,center)
    scene.add(pyrender.OrthographicCamera(xmag=scale*w/h,ymag=scale,zfar=1000),pose=pose)
    for a,e,power in [(-60,60,2.6),(130,30,1.1),(-150,15,.65)]:
        scene.add(pyrender.DirectionalLight(color=np.ones(3),intensity=power),
                  pose=camera_pose(a,e,center))
    renderer=pyrender.OffscreenRenderer(w,h)
    pixels,_=renderer.render(scene,flags=pyrender.RenderFlags.SHADOWS_DIRECTIONAL)
    renderer.delete()
    return Image.fromarray(pixels)


def main(parts=None,report=None):
    if report is None:
        report=json.loads((OUT/'fit-check.json').read_text())
    if parts is None:
        parts=build(report['screen_tilt_deg'],report['battery_tilt_deg'])
    sheet=Image.new('RGB',(1600,1000),'#f0f2f4')
    draw=ImageDraw.Draw(sheet)
    def text(x,y,message,size=22,bold=False,color='#253038'):
        draw.text((x,y),message,font=ImageFont.truetype(BOLD if bold else FONT,size),fill=color)
    text(55,30,'Rounded top aligned with the screen.',38,True)
    text(57,86,'The crown follows the screen angle; the battery remains upright',23,color='#5a6972')
    sheet.paste(render(parts,['body','bezel','lid','board','active'],(940,680)),(10,150))
    sheet.paste(render(parts,['body','bezel','lid','board','battery'],(630,660),
                       azimuth=0,elevation=0,center=(0,5,34),scale=40,section=True),(950,175))
    draw=ImageDraw.Draw(sheet)
    text(994,150,'SIDE CUTAWAY',19,True)
    text(994,808,'Blue: battery  /  Green: populated board',19)
    text(994,842,f"Screen: {report['screen_tilt_deg']:g}°  ·  Battery: {report['battery_tilt_deg']:g}°",21,True)
    text(994,875,f"Roof slopes back at {report['screen_tilt_deg']:g}° to match.",19)
    w,depth,height=report['dimensions_mm']
    text(58,846,f'{w:g} × {depth:g} mm footprint  ·  {height:g} mm tall',25,True)
    text(58,890,f"{report['footprint_reduction_percent']:g}% less footprint than the first stand",23)
    text(58,954,'ASSEMBLY STUDY — removable front bezel; battery access underneath. Final fastening and print details pending.',18,color='#66747d')
    sheet.save(OUT/'compact_preview.png')

    exploded=render(parts,['body','bezel','board'],(1400,850),azimuth=-30,elevation=20,
                    center=(0,-15,35),scale=62,exploded=True)
    d=ImageDraw.Draw(exploded)
    d.text((35,25),'Front assembly: bezel → display module → enclosure',
           font=ImageFont.truetype(BOLD,27),fill='#253038')
    d.text((35,805),'Remove the bezel to insert/service the complete module. The pedestal no longer obstructs access.',
           font=ImageFont.truetype(FONT,20),fill='#253038')
    exploded.save(OUT/'assembly_exploded.png')


if __name__=='__main__':
    main()

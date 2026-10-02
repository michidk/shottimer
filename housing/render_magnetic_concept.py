"""Render base shape with explicitly labeled visual markers."""
import os
os.environ.setdefault('PYOPENGL_PLATFORM','egl')
import numpy as np
np.infty = np.inf
import pyrender
from PIL import Image, ImageDraw, ImageFont
from render_concept import mesh, camera_pose, FONT, BOLD


def render_preview(parts,output,usb,magnet):
    sheet = Image.new('RGB',(1500,1060),'#f0f2f4')
    colors = {'body':[.28,.34,.35,1],'cover':[.22,.28,.29,1],
              'active':[.025,.09,.10,1],'magnet':[.75,.76,.78,1],
              'port':[.04,.04,.04,1],'battery':[.20,.48,.72,1]}
    views = [('Front / right side',-45,20,['body','cover','active','magnet','port']),
             ('Front / left side',-135,20,['body','cover','active','magnet','port']),
             ('Rear open shell',45,25,['body']),
             ('Battery envelope inside shell',135,25,['body','battery'])]
    for index,(label,azimuth,elevation,names) in enumerate(views):
        scene = pyrender.Scene(bg_color=[.94,.95,.96,1],ambient_light=[.5,.5,.5])
        for name in names:
            material = pyrender.MetallicRoughnessMaterial(baseColorFactor=colors[name],roughnessFactor=.75)
            scene.add(pyrender.Mesh.from_trimesh(mesh(parts[name]),material=material,smooth=False))
        center = (0,0,29)
        scene.add(pyrender.OrthographicCamera(xmag=53,ymag=35,zfar=1000),pose=camera_pose(azimuth,elevation,center))
        for az,el,intensity in [(-60,60,2.6),(130,30,1.1),(-150,15,.65)]:
            scene.add(pyrender.DirectionalLight(color=np.ones(3),intensity=intensity),pose=camera_pose(az,el,center))
        renderer = pyrender.OffscreenRenderer(750,440)
        pixels,_ = renderer.render(scene)
        renderer.delete()
        x,y = (index%2)*750,100+(index//2)*465
        sheet.paste(Image.fromarray(pixels),(x,y))
        ImageDraw.Draw(sheet).text((x+25,y),label,font=ImageFont.truetype(FONT,22),fill='#253038')
    draw = ImageDraw.Draw(sheet)
    draw.text((25,15),'Magnetic enclosure — base shape review',font=ImageFont.truetype(BOLD,30),fill='#253038')
    draw.text((25,58),f'48 W × 58 H × 28 D mm | USB {usb} | Ø12 × 2.7 mm magnet {magnet} | no stand',font=ImageFont.truetype(FONT,21),fill='#253038')
    draw.text((25,1025),'CONCEPT: screen, USB and magnet are markers. Cutouts, magnet pocket and retainers come next.',font=ImageFont.truetype(FONT,20),fill='#253038')
    sheet.save(output)

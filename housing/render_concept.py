"""Render the actual CadQuery shape study with visual-only screen/battery proxies."""
import os
os.environ.setdefault('PYOPENGL_PLATFORM', 'egl')
from pathlib import Path
import math
import json
import numpy as np
# pyrender 0.1.45 still uses this NumPy 1.x alias.
np.infty = np.inf
import trimesh
import pyrender
from PIL import Image, ImageDraw, ImageFont
from model import build

OUT=Path(__file__).parent/'output'
FONT='/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'
BOLD='/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'


def mesh(shape):
    v,f=shape.val().tessellate(0.02,0.1)
    return trimesh.Trimesh(vertices=[p.toTuple() for p in v], faces=f, process=True)


def camera_pose(azimuth,elevation,center,distance=250):
    a,e=map(math.radians,(azimuth,elevation))
    eye=np.array([math.cos(e)*math.cos(a),math.cos(e)*math.sin(a),math.sin(e)])
    right=np.cross([0,0,1],eye); right/=np.linalg.norm(right)
    up=np.cross(eye,right)
    pose=np.eye(4)
    pose[:3,:3]=np.column_stack([right,up,eye])
    pose[:3,3]=np.array(center)+distance*eye
    return pose


def render(names,azimuth,elevation,center,scale,size):
    scene=pyrender.Scene(bg_color=[0.94,0.95,0.96,1],ambient_light=[.55,.55,.55])
    colors={'tray':[.25,.29,.31,1], 'upper':[.38,.44,.46,1],
            'face':[.025,.03,.04,1], 'active':[.015,.12,.15,1],
            'battery':[.18,.42,.67,1]}
    tilt=json.loads((OUT/'dimensions.json').read_text())['tilt_deg']
    parts=build(tilt)
    for name in names:
        material=pyrender.MetallicRoughnessMaterial(baseColorFactor=colors[name],roughnessFactor=.72)
        scene.add(pyrender.Mesh.from_trimesh(mesh(parts[name]),material=material,smooth=False))
    ground=trimesh.creation.box(extents=[250,250,.2])
    ground.apply_translation([0,0,-.2])
    scene.add(pyrender.Mesh.from_trimesh(ground,material=pyrender.MetallicRoughnessMaterial(
        baseColorFactor=[.88,.90,.92,1],roughnessFactor=1)))
    pose=camera_pose(azimuth,elevation,center)
    w,h=size
    scene.add(pyrender.OrthographicCamera(xmag=scale*w/h,ymag=scale,zfar=1000),pose=pose)
    for a,e,power in [(-50,60,2.4),(130,40,1.2),(-140,20,.6)]:
        scene.add(pyrender.DirectionalLight(color=np.ones(3),intensity=power),
                  pose=camera_pose(a,e,center))
    renderer=pyrender.OffscreenRenderer(w,h)
    color,_=renderer.render(scene,flags=pyrender.RenderFlags.SHADOWS_DIRECTIONAL)
    renderer.delete()
    return Image.fromarray(color)


def main():
    OUT.mkdir(exist_ok=True)
    report=json.loads((OUT/'dimensions.json').read_text())
    sheet=Image.new('RGB',(1600,1050),'#f0f2f4')
    d=ImageDraw.Draw(sheet)
    def text(x,y,s,size=22,bold=False,color='#253038'):
        d.text((x,y),s,font=ImageFont.truetype(BOLD if bold else FONT,size),fill=color)
    text(55,32,'Coffee display stand',38,True)
    text(57,85,f"First shape study  /  {report['tilt_deg']:g}° tilt back from vertical",23,color='#5a6972')
    sheet.paste(render(['tray','upper','face','active'],-55,25,[0,0,35],53,(960,710)),(15,140))
    sheet.paste(render(['tray','upper','face','active'],0,3,[0,0,37],46,(600,425)),(985,135))
    sheet.paste(render(['tray','battery'],-55,53,[0,0,8],39,(600,330)),(985,590))
    d=ImageDraw.Draw(sheet)
    text(1015,133,'SIDE VIEW',18,True)
    text(1015,575,'BATTERY BASE — COVER REMOVED',18,True)
    text(1015,905,'50 × 34 × 10 mm pack shown in blue',18)
    w,depth,h=report['overall_mm']
    text(60,863,f'{w:g} mm wide  ×  {depth:g} mm deep  ×  {h:.0f} mm tall',25,True)
    text(60,906,'Battery stays low; USB-C is planned on the right of the head.',20)
    text(60,938,'Tilt is set in the model before printing. No moving joint.',20)
    text(60,995,'CONCEPT ONLY — port, screen opening, retention, wire channel and finishing come next.',18,color='#727d84')
    sheet.save(OUT/'concept.png')


if __name__=='__main__':
    main()

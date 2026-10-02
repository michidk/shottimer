"""USB-side assembly view matching the user's printed prototype photo."""
import os,math,json
os.environ['PYOPENGL_PLATFORM']='egl'
import numpy as np
np.infty=np.inf
import cadquery as cq,pyrender
from PIL import Image,ImageDraw,ImageFont
from render_concept import mesh,camera_pose,FONT,BOLD
import usb_centered_stand as m
p=globals().get('parts') or m.build();out=m.OUT;out.mkdir(parents=True,exist_ok=True)
scene=pyrender.Scene(bg_color=[.88,.89,.90,1],ambient_light=[.5,.5,.5])
def add(shape,color):
 material=pyrender.MetallicRoughnessMaterial(baseColorFactor=color,roughnessFactor=.8,doubleSided=True)
 scene.add(pyrender.Mesh.from_trimesh(mesh(shape),material=material,smooth=False))
for n in ('body','cover'):add(p[n],[.14,.16,.17,1])
add(p['battery'],[.65,.49,.21,1])
for i,s in enumerate(p['board'].solids().vals()):
 color=[.09,.36,.30,1] if i==0 else ([.025,.035,.045,1] if i==116 or i==149 else [.72,.74,.76,1])
 add(cq.Workplane('XY').newObject([s]),color)
for az,el,power in [(0,15,2.0),(-65,55,1.1),(80,35,.8)]:
 scene.add(pyrender.DirectionalLight(color=np.ones(3),intensity=power),pose=camera_pose(az,el,(15,8,50)))
sheet=Image.new('RGB',(1600,1050),'#e0e3e6')
for i,(az,el,scale,center) in enumerate(((-8,12,29,(15,10,49)),(0,0,12,(22,9,47)))):
 pose=camera_pose(az,el,center)
 a=math.radians(-20);rotation=np.array([[math.cos(a),-math.sin(a),0],[math.sin(a),math.cos(a),0],[0,0,1]])
 pose[:3,:3]=pose[:3,:3]@rotation
 cam=scene.add(pyrender.OrthographicCamera(xmag=scale*800/900,ymag=scale,zfar=1000),pose=pose)
 renderer=pyrender.OffscreenRenderer(800,900);pixels,_=renderer.render(scene);renderer.delete();scene.remove_node(cam)
 sheet.paste(Image.fromarray(pixels),(i*800,80))
d=ImageDraw.Draw(sheet)
d.text((30,22),'Corrected CAD — USB opening centered on the connector',font=ImageFont.truetype(BOLD,29),fill='#253038')
d.text((30,993),'12 × 7 mm opening, R0.8 corners; moved 2.88 mm toward the display.',font=ImageFont.truetype(FONT,23),fill='#253038')
sheet.save(out/'usb_photo_comparison.png')
usb=p['board_local'].solids().vals()[80].BoundingBox()
report={'opening_depth_interval_mm':[m.usb_back-m.usb_rearward_mm,m.usb_front-m.usb_rearward_mm],
        'usb_shell_depth_interval_mm':[usb.zmin,usb.zmax],
        'shell_extent_forward_of_opening_mm':usb.zmax-(m.usb_front-m.usb_rearward_mm),
        'note':'Intervals measured normal to display plane. Shell extent is not the internal connector aperture.'}
(out/'usb-photo-study.json').write_text(json.dumps(report,indent=2)+'\n')
print(report,flush=True)

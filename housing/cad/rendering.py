"""Shared offscreen CAD rendering; independent of either enclosure model."""
import os
os.environ.setdefault('PYOPENGL_PLATFORM', 'egl')
import math
import numpy as np
np.infty = np.inf  # pyrender 0.1.45 compatibility
import trimesh
import pyrender
from PIL import Image

FONT = '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'
BOLD = '/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'

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


def render(parts,names,size,azimuth=-55,elevation=23,center=(0,4,33),scale=46,ground=True):
    scene=pyrender.Scene(bg_color=[.94,.95,.96,1],ambient_light=[.5,.5,.5])
    colors={'body':[.28,.34,.35,1], 'bezel':[.25,.31,.32,1], 'lid':[.23,.28,.30,1],
            'board':[.12,.35,.30,1], 'active':[.035,.20,.23,1], 'battery':[.23,.51,.77,1],
            'gasket':[.07,.09,.1,1]}
    for name in names:
        shape=parts[name]
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

"""Export pad-removal revision after unchanged front-body insertion checks passed."""
import json
import cadquery as cq
import trimesh
import aligned_holders_stand as m
import compact_usb_stand as previous
p=m.build()
old=previous.build(False)
# New rear cover is a strict subset: removing pads cannot create interference.
assert m.overlap(p['cover'],old['cover'])>0
assert p['cover'].cut(old['cover']).val().Volume()<1e-5
removed=old['cover'].cut(p['cover']).val().Volume()
assert removed>1
for name in ('body','cover_print'):
    assert p[name].val().isValid() and len(p[name].solids().vals())==1
    path=m.OUT/(name+'.stl')
    cq.exporters.export(p[name],str(path),tolerance=.01,angularTolerance=.1)
    mesh=trimesh.load(path,force='mesh');assert mesh.is_watertight and mesh.is_volume
assy=cq.Assembly()
for name in ('body','cover','board','battery'):assy.add(p[name],name=name)
assy.export(str(m.OUT/'assembly.step'))
report=json.loads((m.OUT/'fit-check.json').read_text())
report.update({'usb_side_blocks_removed':True,'holder_base_front_z_mm':m.guide_front,
               'raised_rear_contact_pads_removed':True,'removed_pad_volume_mm3':removed,
               'validation':'Full assembly and insertion checks passed before pad removal; final cover is a strict material subset of that checked cover. Final meshes are valid and watertight.'})
(m.OUT/'fit-check.json').write_text(json.dumps(report,indent=2)+'\n')
from render_aligned_holders import main
main(p)
print('Final flat-ended cover exported and rendered',flush=True)

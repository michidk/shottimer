"""Refresh repository documentation images from the current CAD assembly."""
from pathlib import Path
import shutil
import usb_centered_stand as model
from render_compact import render
from render_housing_views import render_views

if __name__=='__main__':
    p=model.build()
    images=Path(__file__).resolve().parent.parent/'.github/images'
    images.mkdir(parents=True,exist_ok=True)
    render(dict(p,bezel=p['cover']),['body','bezel','board','battery','active'],(1200,1000),
           azimuth=-45,elevation=23,center=(0,6,36),scale=50).save(images/'housing-assembly.png')
    render_views(p,model.OUT/'body_views.png')
    for source,target in [('body_views.png','housing-views.png'),('usb_photo_comparison.png','housing-usb.png')]:
        shutil.copy2(model.OUT/source,images/target)

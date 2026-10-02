"""Render front-facing documentation images of both current housings."""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
from cad import stand, magnetic
from cad.rendering import render, FONT, BOLD
from cad.previews import render_stand_views


def main():
    images=Path(__file__).resolve().parent.parent/'.github/images'
    images.mkdir(parents=True,exist_ok=True)
    desktop=stand.build()
    magnetic_parts=magnetic.build()
    desktop=dict(desktop,lid=desktop['cover'])
    magnetic_parts=dict(magnetic_parts,lid=magnetic_parts['cover'],gasket=magnetic_parts['magnet'])
    # Show the display and overall silhouette, from the side opposite each USB port.
    desktop_view=render(desktop,['body','lid','board','active'],(1000,900),
                        azimuth=-115,elevation=22,center=(0,6,36),scale=50,ground=False)
    magnetic_view=render(magnetic_parts,['body','lid','active','gasket'],(1000,900),
                         azimuth=-70,elevation=15,center=(0,0,29),scale=42,ground=False)
    desktop_view.save(images/'housing-assembly.png')
    sheet=Image.new('RGB',(2000,1000),'#f0f2f4')
    sheet.paste(desktop_view,(0,100));sheet.paste(magnetic_view,(1000,100))
    draw=ImageDraw.Draw(sheet)
    for x,title,subtitle in ((40,'Desktop stand','Tilted display · removable press-fit backplate'),
                             (1040,'Magnetic housing','Flat enclosure · glued side magnet · no stand')):
        draw.text((x,18),title,font=ImageFont.truetype(BOLD,34),fill='#253038')
        draw.text((x,65),subtitle,font=ImageFont.truetype(FONT,22),fill='#53626a')
    sheet.save(images/'housing-overview.png')
    render_stand_views(desktop,images/'housing-views.png')
    print('Updated front-facing README images.',flush=True)


if __name__=='__main__':
    main()

"""Review sheets for the two current models."""
from PIL import Image, ImageDraw, ImageFont
from .rendering import render, FONT, BOLD
from . import magnetic as m

def render_stand_views(parts, output):
    views=[('Isometric',-45,25),('Front',-90,20),('USB side',0,0),
           ('Rear',90,0),('Top',-90,89),('Bottom',-90,-89)]
    sheet=Image.new('RGB',(1500,1100),'#f0f2f4')
    for i,(label,az,el) in enumerate(views):
        picture=render(parts,['body'],(500,500),az,el,(0,6,36),50,ground=False)
        x=(i%3)*500;y=(i//3)*550
        sheet.paste(picture,(x,y+40))
        ImageDraw.Draw(sheet).text((x+20,y+10),label,font=ImageFont.truetype(FONT,24),fill='#253038')
    sheet.save(output)


def render_magnetic_preview(parts, output, usb, magnet):
    # The shared renderer's palette calls the cover 'lid' and magnet 'gasket'.
    p = dict(parts, lid=parts['cover'], gasket=parts['magnet'])
    detached = dict(p, lid=parts['cover'].translate((0,-24,0)))
    sheet = Image.new('RGB',(1600,1180),'#f0f2f4')
    views = [('Assembled / magnet side', p, ['body','lid','active','gasket'], -45,20),
             ('Assembled / USB side', p, ['body','lid','active','gasket'], -135,20),
             ('Backplate inside: rim, ribs, battery cradle', detached, ['lid'], -65,23),
             ('Backplate with battery envelope', detached, ['lid','battery'], -135,20)]
    # Move battery with the detached backplate for its assembled view.
    views[3] = (*views[3][:1],dict(detached,battery=p['battery_max'].translate((0,-24,0))),*views[3][2:])
    for i,(label,view,names,az,el) in enumerate(views):
        center = (0,-16 if i>=2 else 0,m.height/2)
        picture = render(view,names,(800,480),az,el,center,37,ground=False)
        x,y = (i%2)*800,100+(i//2)*505
        sheet.paste(picture,(x,y+25))
        ImageDraw.Draw(sheet).text((x+25,y),label,font=ImageFont.truetype(FONT,20),fill='#253038')
    d = ImageDraw.Draw(sheet)
    d.text((25,15),'Magnetic housing — press-fit backplate',font=ImageFont.truetype(BOLD,30),fill='#253038')
    d.text((25,60),f'{m.width:g} × {m.height:g} × {m.depth:g} mm | USB {usb} | magnet {magnet} | {m.rim_depth:g} mm rim + four friction ribs',font=ImageFont.truetype(FONT,22),fill='#253038')
    d.text((25,1135),'Feature prototype: see fit-check.json; printed fit, glue bond and cable overmould need physical checking.',font=ImageFont.truetype(FONT,20),fill='#253038')
    sheet.save(output)

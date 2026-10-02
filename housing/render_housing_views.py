"""Six CAD views, using the project's own offscreen renderer."""
from PIL import Image, ImageDraw, ImageFont
from render_compact import render
from render_concept import FONT

def render_views(parts, output):
    views=[('Isometric',-45,25),('Front',-90,20),('USB side',0,0),
           ('Rear',90,0),('Top',-90,89),('Bottom',-90,-89)]
    sheet=Image.new('RGB',(1500,1100),'#f0f2f4')
    for i,(label,az,el) in enumerate(views):
        picture=render(parts,['body'],(500,500),az,el,(0,6,36),50,ground=False)
        x=(i%3)*500;y=(i//3)*550
        sheet.paste(picture,(x,y+40))
        ImageDraw.Draw(sheet).text((x+20,y+10),label,font=ImageFont.truetype(FONT,24),fill='#253038')
    sheet.save(output)

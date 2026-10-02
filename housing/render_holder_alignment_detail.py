import math
from PIL import Image,ImageDraw,ImageFont
from render_compact import render,FONT,BOLD
import curved_holder_stand as new
import aligned_holders_stand as old
sheet=Image.new('RGB',(1600,950),'#f0f2f4')
for i,(m,label) in enumerate(((old,'BEFORE: flat base, separate recess'),(new,'AFTER: base follows the recess curve'))):
 p=m.build(False)
 fy=m.front_base_y+m.screen_center_z*math.tan(math.radians(m.screen_tilt_deg))
 local=p['body'].translate((0,-fy,-m.screen_center_z)).rotate((0,0,0),(1,0,0),-(90-m.screen_tilt_deg))
 crop=local.intersect(m.box(10,8,10,(0,-18.4,-3.5)))
 shown=crop.rotate((0,0,0),(1,0,0),180).translate((0,0,10))
 pic=render(dict(body=shown),['body'],(800,800),azimuth=-65,elevation=35,center=(0,18.4,13.5),scale=6.5)
 sheet.paste(pic,(800*i,90))
 ImageDraw.Draw(sheet).text((800*i+25,25),label,font=ImageFont.truetype(BOLD,25),fill='#253038')
ImageDraw.Draw(sheet).text((35,906),'Centre lower finger — same scale and viewpoint. PCB-catching lip retained.',font=ImageFont.truetype(FONT,23),fill='#253038')
new.OUT.mkdir(exist_ok=True,parents=True)
sheet.save(new.OUT/'holder_alignment_detail.png')

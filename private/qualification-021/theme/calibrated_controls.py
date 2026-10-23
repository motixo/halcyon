#!/usr/bin/python3
"""Read-only screenshot calibration; no native settings or palette writes.
Fixed bounds come from preserved 1280x800 full-session screenshots. Actions are
allowed only after their visual image pattern is revalidated on the live screen.
"""
import hashlib,json,pathlib,subprocess

class Pixels:
 def __init__(self,path):
  self.path=pathlib.Path(path)
  try:
   from PIL import Image
   im=Image.open(path).convert('RGB');self.width,self.height=im.size;self.data=im.tobytes()
  except ImportError:
   self.width,self.height=map(int,subprocess.check_output(['identify','-format','%w %h',str(path)]).split())
   self.data=subprocess.check_output(['convert',str(path),'-depth','8','rgb:-'],timeout=20)
  if (self.width,self.height)!=(1280,800):raise ValueError('Uncalibrated screen dimensions')
 def rgb(self,x,y):
  i=(y*self.width+x)*3;return tuple(self.data[i:i+3])

LOOK_BOUNDS={'calm':[519,260,750,413],'evening':[762,260,993,413],
             'focus':[275,426,506,570],'paper':[519,426,750,570],'rose':[762,426,993,570]}
MODE_BOUNDS={'Dark':[578,297,768,397],'Light':[800,297,990,397],'Auto':[953,451,1003,476]}

def validate_samples(pixels,samples,tolerance=12,minimum=.90):
 matches=sum(max(abs(a-b) for a,b in zip(pixels.rgb(*p['xy']),p['rgb']))<=tolerance for p in samples)
 if matches<len(samples)*minimum:raise ValueError(f'Live screenshot differs from calibrated control: {matches}/{len(samples)} samples')
 return {'matched_samples':matches,'total_samples':len(samples),'tolerance':tolerance}

def point(path,control,calibration):
 p=Pixels(path)
 if control in LOOK_BOUNDS:
  detail=validate_samples(p,calibration['wizard_samples'])
  box=LOOK_BOUNDS[control]
 elif control=='Start':
  detail=validate_samples(p,calibration['welcome_samples'])
  box=[279,431,411,471]
 else:
  if control not in MODE_BOUNDS:raise ValueError('Unknown calibrated control '+control)
  # Interior of the actual Dark and Light illustration windows, independent
  # of native tint and OCR segmentation. No labels or wallpaper satisfy this.
  dark=[p.rgb(x,y) for x,y in [(620,350),(730,350),(620,375),(730,375)]]
  light=[p.rgb(x,y) for x,y in [(830,350),(950,350),(830,375),(950,375)]]
  if not all(max(c)-min(c)<12 and max(c)<65 for c in dark):raise ValueError('Dark image control pattern absent')
  if not all(max(c)-min(c)<12 and min(c)>160 for c in light):raise ValueError('Light image control pattern absent')
  detail={'dark_image_pixels':dark,'light_image_pixels':light}
  box=MODE_BOUNDS[control]
  if control=='Auto':
   left=p.rgb(965,463);right=p.rgb(990,463)
   if abs(sum(left)-sum(right))<75:raise ValueError('Automatic toggler thumb/track pattern absent')
   detail.update(toggle_left=left,toggle_right=right,enabled=sum(right)<sum(left))
 x,y,right,bottom=box
 detail.update(control=control,bounds=box,point=[(x+right)//2,(y+bottom)//2],screenshot_sha256=hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest())
 return detail

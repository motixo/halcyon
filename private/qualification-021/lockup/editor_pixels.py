"""Fresh screenshot calibration for core editor focus only, never native geometry."""
import re
from calibrated_controls import Pixels

def locate(path,words,identity_seen=False):
 p=Pixels(path);base=p.rgb(640,400)
 def near(c,limit=5):return max(abs(a-b) for a,b in zip(c,base))<=limit
 # Retained native editor's large flat document surface crosses the screen
 # center. Reject wallpaper, bars and small popups. This fallback is disabled
 # for optional move/resize assertions and revalidated on EVERY use.
 left=right=640
 while left>10 and near(p.rgb(left-1,400)):left-=1
 while right<1270 and near(p.rgb(right+1,400)):right+=1
 if not 500<=right-left<=1200:raise ValueError('No calibrated large editor document surface at screen center')
 def chrome(c):return near(c,70) or (max(c)-min(c)<16 and max(c)<120)
 top=bottom=400
 while top>45 and chrome(p.rgb(640,top-1)):top-=1
 while bottom<715 and chrome(p.rgb(640,bottom+1)):bottom+=1
 if not (80<=top<=220 and 480<=bottom<=710 and bottom-top>=280):raise ValueError('Editor frame bounds differ from retained native layout')
 text=''.join(re.sub('[^a-z0-9]','',w['text'].lower()) for w in words)
 filename='lockupinputtxt' in text
 if not identity_seen and not filename:raise ValueError('Neither native editor identity nor visible filename validates pixel focus')
 # Header/tab must contain real foreground contrast; the body alone is not
 # identification. Fresh frame is retained for independent human inspection.
 ink=sum(max(abs(a-b) for a,b in zip(p.rgb(x,y),base))>65 for y in range(top+15,top+85) for x in range(left+15,min(right,left+260)))
 if ink<100:raise ValueError('Native editor header/tab foreground absent')
 return {'x':left-4,'y':top-4,'width':right-left+9,'height':bottom-top+9,'source':'fresh screenshot calibrated focus region; not protocol geometry','filename_ocr_seen':filename,'header_foreground_pixels':ink,'document_rgb':base}

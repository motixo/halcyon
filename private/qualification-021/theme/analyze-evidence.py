#!/usr/bin/python3
"""Read-only analysis of real screenshots against recorded native keys.
This never emits or installs palettes. A pixel match is evidence, not automatic
release qualification. Clock bounds are observed text bounds, not wallpaper.
"""
import json,pathlib,re,sys,math
from collections import Counter
import subprocess

def image_pixels(path,box):
 x,y,right,bottom=box
 raw=subprocess.check_output(['convert',str(path),'-crop',f'{right-x}x{bottom-y}+{x}+{y}','-depth','8','rgb:-'],timeout=15)
 return list(zip(raw[0::3],raw[1::3],raw[2::3]))

def color(raw,role):
 m=re.search(r'^    '+role+r':\s*"#([0-9A-Fa-f]{6})[0-9A-Fa-f]{2}"',raw,re.M)
 if m:return tuple(bytes.fromhex(m[1]))
 m=re.search(r'^    '+role+r':\s*\(\s*red: ([0-9.eE+-]+),\s*green: ([0-9.eE+-]+),\s*blue: ([0-9.eE+-]+)',raw,re.M)
 if m:return tuple(round(float(v)*255) for v in m.groups())

def near(a,b,t=35):return sum((x-y)**2 for x,y in zip(a,b))<=t*t

def luminance(rgb):
 linear=[v/255/12.92 if v/255<=.04045 else ((v/255+.055)/1.055)**2.4 for v in rgb]
 return sum(a*b for a,b in zip(linear,[.2126,.7152,.0722]))

def analyze(root):
 summary=json.loads((root/'theme-results.json').read_text());out=[]
 for f in summary['frames']:
  state=json.loads((root/(f['name']+'-state.txt')).read_text());files=state['files']
  def raw(ending):
   return next((bytes.fromhex(v['hex']).decode(errors='replace') for k,v in files.items() if k.endswith(ending)),None)
  dark=raw('CosmicTheme.Mode/v1/is_dark')
  portal=(root/(f['name']+'-portal.txt')).read_text().splitlines()[0]
  effective_dark=(dark.strip()=='true') if dark is not None else bool(re.search(r'\bu 1$',portal))
  variant='Dark' if effective_dark else 'Light'
  palette=raw('CosmicTheme.'+variant+'/v2/background') or raw('CosmicTheme.'+variant+'/v1/background')
  row={'frame':f['name'],'variant':variant,'stopped':f['stopped'],'bar_background_policy':{b:raw('CosmicPanel.'+b+'/v1/background') for b in ['Panel','Dock']}}
  if palette:
   expected_bg=color(palette,'base');expected_fg=color(palette,'on')
   row.update(expected_native_background=expected_bg,expected_native_foreground=expected_fg)
   metrics=f['pixels'].get('clock_text')
   if metrics and expected_bg and expected_fg:
    pixels=image_pixels(root/(f['name']+'.png'),tuple(metrics['bounds']))
    bg=sum(near(v,expected_bg,16) for v in pixels);fg=sum(near(v,expected_fg,35) for v in pixels)
    dominant=Counter(pixels).most_common(1)[0][0]
    foregrounds=[v for v in pixels if near(v,expected_fg,35)]
    observed_fg=Counter(foregrounds).most_common(1)[0][0] if foregrounds else None
    contrast=(max(luminance(dominant),luminance(observed_fg))+.05)/(min(luminance(dominant),luminance(observed_fg))+.05) if observed_fg else None
    row['text_crop']={'observed_surface':dominant,'observed_foreground':observed_fg,'observed_contrast':contrast,'bounds':metrics['bounds'],'area':len(pixels),'background_pixels':bg,'foreground_pixels':fg,'palette_rgb_match':bg>30 and fg>12,'visible_text_contrast':fg>12 and contrast is not None and contrast>=4.5,'dominant':Counter(pixels).most_common(8)}
  procs=state['processes']
  row['settings_daemon']=[p for p in procs if p['exe']=='/usr/bin/cosmic-settings-daemon']
  row['portal']=[p for p in procs if p['exe']=='/usr/libexec/xdg-desktop-portal-cosmic']
  row['halcyon_config']={k:json.loads(bytes.fromhex(v['hex'])) for k,v in files.items() if k=='.config/halcyon/config.json'}
  out.append(row)
 return {'root':str(root),'lineage':summary['lineage'],'host':summary['host'],'errors':summary['errors'],'frames':out,'scenarios':summary['scenarios'],'automatic':summary['automatic']}

if __name__=='__main__':
 for arg in sys.argv[1:]:
  root=pathlib.Path(arg)
  result=analyze(root)
  (root/'independent-pixel-analysis.json').write_text(json.dumps(result,indent=2)+'\n')
  for f in result['frames']:print(root.name,f['frame'],f.get('text_crop'))

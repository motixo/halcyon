#!/usr/bin/python3
"""Read-only analysis of real screenshots against recorded native keys.
This never emits or installs palettes. A pixel match is evidence, not automatic
release qualification. Clock bounds are observed text bounds, not wallpaper.
"""
import json,pathlib,re,sys,math
from collections import Counter
from PIL import Image

def color(raw,role):
 m=re.search(r'^    '+role+r':\s*"#([0-9A-Fa-f]{6})[0-9A-Fa-f]{2}"',raw,re.M)
 if m:return tuple(bytes.fromhex(m[1]))
 m=re.search(r'^    '+role+r':\s*\(\s*red: ([0-9.eE+-]+),\s*green: ([0-9.eE+-]+),\s*blue: ([0-9.eE+-]+)',raw,re.M)
 if m:return tuple(round(float(v)*255) for v in m.groups())

def near(a,b,t=35):return sum((x-y)**2 for x,y in zip(a,b))<=t*t

def analyze(root):
 summary=json.loads((root/'theme-results.json').read_text());out=[]
 for f in summary['frames']:
  state=json.loads((root/(f['name']+'-state.txt')).read_text());files=state['files']
  def raw(ending):
   return next((bytes.fromhex(v['hex']).decode(errors='replace') for k,v in files.items() if k.endswith(ending)),None)
  dark=raw('CosmicTheme.Mode/v1/is_dark');variant='Dark' if dark and dark.strip()=='true' else 'Light'
  palette=raw('CosmicTheme.'+variant+'/v2/background') or raw('CosmicTheme.'+variant+'/v1/background')
  row={'frame':f['name'],'variant':variant,'stopped':f['stopped'],'bar_background_policy':{b:raw('CosmicPanel.'+b+'/v1/background') for b in ['Panel','Dock']}}
  if palette:
   expected_bg=color(palette,'base');expected_fg=color(palette,'on')
   row.update(expected_native_background=expected_bg,expected_native_foreground=expected_fg)
   metrics=f['pixels'].get('clock_text')
   if metrics and expected_bg and expected_fg:
    pixels=list(Image.open(root/(f['name']+'.png')).convert('RGB').crop(tuple(metrics['bounds'])).getdata())
    bg=sum(near(v,expected_bg,16) for v in pixels);fg=sum(near(v,expected_fg,35) for v in pixels)
    row['text_crop']={'bounds':metrics['bounds'],'area':len(pixels),'background_pixels':bg,'foreground_pixels':fg,'mode_coherent':bg>30 and fg>12,'dominant':Counter(pixels).most_common(8)}
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

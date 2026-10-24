#!/usr/bin/python3
"""Controller-only independent observation of real glyph/surface pixels.
Fixed clock-label regions are bounded inside the actual native bar, not wallpaper.
Original screenshot/OCR metrics remain unmodified.
"""
import importlib.util,json,pathlib,sys
from collections import Counter
spec=importlib.util.spec_from_file_location('pixels',pathlib.Path(__file__).parents[1]/'analyze-evidence.py')
a=importlib.util.module_from_spec(spec);spec.loader.exec_module(a)
for arg in sys.argv[1:]:
 root=pathlib.Path(arg);r=json.loads((root/'theme-results.json').read_text());rows=[]
 box=(510,17,611,30) if r['host']=='islands' else (590,17,691,30)
 for frame in r['frames']:
  name=frame['name'];state=json.loads((root/(name+'-state.txt')).read_text());files=state['files']
  def raw(ending):return next((bytes.fromhex(v['hex']).decode(errors='replace') for k,v in files.items() if k.endswith(ending)),None)
  portal=(root/(name+'-portal.txt')).read_text().splitlines()[0]
  dark='u 1' in portal;variant='Dark' if dark else 'Light'
  native=raw('CosmicTheme.'+variant+'/v2/background') or raw('CosmicTheme.'+variant+'/v1/background')
  pixels=a.image_pixels(root/(name+'.png'),box);bg=Counter(pixels).most_common(1)[0][0]
  role=a.color(native,'on') if native else None
  matching=[p for p in pixels if role and a.near(p,role,35)]
  fg=Counter(matching).most_common(1)[0][0] if matching else None
  contrast=(max(a.luminance(bg),a.luminance(fg))+.05)/(min(a.luminance(bg),a.luminance(fg))+.05) if fg else None
  rows.append({'frame':name,'portal':portal,'variant':variant,'inside_bar_clock_group':box,'area':len(pixels),'observed_surface':bg,'expected_native_foreground_role':role,'observed_glyph_foreground':fg,'near_role_glyph_pixels':len(matching),'observed_contrast':contrast,'actual_dominants':Counter(pixels).most_common(8),'stopped_ui':frame['stopped']})
 result={'method':'independent PNG crop at recorded clock-label bounds; compare observed surface/glyph pixels with native role in recorded config; no config writes or screenshot mutation','root':str(root),'phase':r.get('phase'),'host':r['host'],'observations':rows}
 (root/'independent-full-clock-review.json').write_text(json.dumps(result,indent=2)+'\n')
 print(json.dumps({'root':str(root),'frames':len(rows),'contrast':[(v['frame'],v['observed_contrast'],v['near_role_glyph_pixels']) for v in rows]},indent=2))

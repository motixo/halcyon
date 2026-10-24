import hashlib,json,pathlib,shutil,subprocess,sys
work,src=map(pathlib.Path,sys.argv[1:3]);ev=work/'evidence';items={}
paths=['theme/control.py','theme/theme_session.py','theme/calibrated_controls.py','theme/followup/control-calibration.json','lockup/wayland_observer.py','lockup/resources.py','lockup/editor_pixels.py','lockup/stop.py','lockup/protocols/cosmic-toplevel-info-unstable-v1.xml','lockup/protocols/ext-foreign-toplevel-list-v1.xml']
paths += ['blur-isolation/'+p.name for p in (src/'blur-isolation').iterdir() if p.is_file()]
for name in paths:
 p=src/name;data=p.read_bytes();items[name]=hashlib.sha256(data).hexdigest();target=ev/'executed-source'/name;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(data)
(ev/'executed-source-sha256.json').write_text(json.dumps(items,indent=2)+'\n')
(ev/'infra-commit.txt').write_text(subprocess.check_output(['git','-C',str(src),'rev-parse','HEAD'],text=True))

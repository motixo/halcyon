#!/usr/bin/python3
"""Guest-only bounded counters; no home contents, credentials or calendar data."""
import json,os,pathlib,time
rows=[]
for p in pathlib.Path('/proc').iterdir():
 if not p.name.isdigit():continue
 try:
  exe=os.readlink(p/'exe')
  if not any(k in exe for k in ['cosmic','halcyon','Xwayland','xdg-desktop-portal']):continue
  fields=(p/'stat').read_text().rsplit(')',1)[1].split();status={}
  for line in (p/'status').read_text().splitlines():
   key,_,value=line.partition(':')
   if key in ['VmRSS','VmSize','Threads','RssAnon','RssFile']:status[key]=value.strip()
  rows.append({'pid':int(p.name),'exe':exe,'utime':int(fields[11]),'stime':int(fields[12]),'starttime':int(fields[19]),'fd_count':len(list((p/'fd').iterdir())),**status})
 except (OSError,ValueError):pass
print(json.dumps({'realtime':time.time(),'monotonic':time.monotonic(),'uptime':pathlib.Path('/proc/uptime').read_text().strip(),'meminfo':pathlib.Path('/proc/meminfo').read_text(),'processes':rows}))

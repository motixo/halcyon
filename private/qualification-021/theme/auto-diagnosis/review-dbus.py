#!/usr/bin/python3
"""Read-only classification of captured real system DBus messages in order."""
import json,pathlib,re,sys
for arg in sys.argv[1:]:
 root=pathlib.Path(arg);text=(root/'final-dbus-traffic.txt').read_text()
 chunks=re.split(r'(?=^(?:method call|method return|error|signal) )',text,flags=re.M)
 messages=[]
 for chunk in chunks:
  if not re.match(r'^(?:method call|method return|error|signal) ',chunk):continue
  header=chunk.splitlines()[0];member=re.search(r'member=([^ ;]+)',header)
  member=member[1] if member else None
  messages.append({'ordinal':len(messages),'header':header,'member':member,'strings':re.findall(r'^\s*string "([^"]*)"',chunk,re.M),'raw':chunk.rstrip()})
 stimuli=[i for i,m in enumerate(messages) if m['member'] in ['SetTime','SetTimezone','SetNTP']]
 intervals=[]
 for pos,index in enumerate(stimuli):
  stop=stimuli[pos+1] if pos+1<len(stimuli) else len(messages)
  following=[m for m in messages[index+1:stop] if m['member']=='PropertiesChanged']
  intervals.append({'stimulus':messages[index],'following_properties_changed_before_next_stimulus':following,'following_timezone_signal':any('Timezone' in m['strings'] for m in following),'following_time_usec_signal':any('TimeUSec' in m['strings'] for m in following)})
 result={'root':str(root),'total_messages':len(messages),'all_properties_changed':[m for m in messages if m['member']=='PropertiesChanged'],'service_owner_changes':[m for m in messages if m['member']=='NameOwnerChanged'],'stimulus_intervals':intervals}
 (root/'independent-dbus-review.json').write_text(json.dumps(result,indent=2)+'\n')
 print(json.dumps({'root':str(root),'messages':len(messages),'properties_changes':len(result['all_properties_changed']),'stimuli':[(v['stimulus']['member'],v['following_timezone_signal'],v['following_time_usec_signal']) for v in intervals]},indent=2))

"""Infrastructure-only evidence framing; never qualification or admission."""
import json,pathlib,re,time
FORMAT='halcyon-trial-command-receipt'
def command_receipt(label,returncode,stdout,stderr,out):
 if not re.fullmatch(r'[A-Za-z0-9_.-]{1,100}',label):raise ValueError('invalid_command_label')
 value={'format':FORMAT,'version':1,'label':label,'returncode':returncode,'stdout':stdout,'stderr':stderr}
 # Prefix isolates receipts from observations; exclusive creation prevents clobber.
 for suffix in range(8):
  name=f'command.{label}.{time.time_ns()}.{suffix}.receipt.json'
  value['receipt_file']=name
  try:
   with (pathlib.Path(out)/name).open('x') as f:json.dump(value,f,indent=2);f.write('\n')
   return value
  except FileExistsError:continue
 raise FileExistsError('receipt_filename_collision')
def device_observation(raw,expected_helpers):
 envelope=json.loads(raw)
 if envelope.get('format')!=FORMAT or envelope.get('version')!=1:raise ValueError('device_receipt_format')
 if envelope.get('returncode')!=0:raise ValueError('device_command_failed')
 if not isinstance(envelope.get('stdout'),str):raise ValueError('device_stdout_type')
 value=json.loads(envelope['stdout'])
 if value.get('format')!='halcyon-drm-device-observation' or value.get('version')!=1:raise ValueError('device_observation_format')
 if value.get('agreeing_snapshots')!=2:raise ValueError('device_snapshot_count')
 if not isinstance(value.get('devices'),list) or not value['devices']:raise ValueError('device_values_missing')
 if not isinstance(value.get('bindings'),list) or not value['bindings']:raise ValueError('device_bindings_missing')
 if value.get('helper_sha256')!=expected_helpers:raise ValueError('device_helper_identity')
 return value

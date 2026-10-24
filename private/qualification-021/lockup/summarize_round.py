#!/usr/bin/python3
"""Recheck downloaded R3 evidence; no VM, app, network or fixture writes."""
import hashlib
import json
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parent
INFRA = ROOT.parents[2]
EXECUTED = '7840d5609b08ef95ac529af6ab546d77ccc36242'
RUNS = {'stock': '40018232557', 'main': '40027094006', 'islands': '40035187607'}
CRITICAL = ['session.py', 'wayland_observer.py', 'resources.py', 'start.sh',
            'stop.py', 'editor_pixels.py', 'live_repair.py']

def sha(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        for block in iter(lambda: f.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()

expected = {name: hashlib.sha256(subprocess.check_output([
    'git', 'show', EXECUTED + ':private/qualification-021/lockup/' + name],
    cwd=INFRA)).hexdigest() for name in CRITICAL}
result = {'executed_infra': EXECUTED, 'runtime_source_sha256': expected,
          'publication': 'ON HOLD', 'rx590_causation': 'unproven', 'contexts': {}}
for cohort, run in RUNS.items():
    download = ROOT / ('evidence-' + run)
    evidence = download / ('lockup-session-' + cohort + '-' + run)
    summary_bytes = subprocess.check_output(['python3', str(ROOT / 'summarize_soak.py'), str(evidence)])
    (ROOT / (cohort + '-summary.json')).write_bytes(summary_bytes)
    summary = json.loads(summary_bytes)
    assert summary['core_input_hour_passed'] and summary['full_native_journal_present']
    stage_sources = sorted(evidence.glob('executed-source-*.json'))
    assert len(stage_sources) >= 5
    for file in stage_sources:
        manifest = json.loads(file.read_text())
        assert all(manifest[name] == digest for name, digest in expected.items()), file
    for name, digest in expected.items():
        assert sha(evidence / 'executed-harness' / name) == digest, name
    artifacts = []
    for item in json.loads((download / 'artifact-metadata.json').read_text()):
        if not item['name'].startswith(('lockup-preflight-', 'lockup-progress-', 'lockup-session-')):
            continue
        archive = download / (item['name'] + '.zip')
        assert 'sha256:' + sha(archive) == item['digest'], archive
        artifacts.append({k: item[k] for k in ['id', 'name', 'size_in_bytes', 'digest', 'expires_at']})
    assert len(artifacts) == 5
    selected = ['lockup-results.json', 'resource-samples.jsonl', 'full-native-journal.txt',
                'native-environment.txt', 'qmp-blocks.json', 'vm-command.txt',
                'editor-unit-0.txt', 'editor-unit-1.txt', 'preflight-identity.txt',
                'core-final-identity.txt', 'final-identity.txt']
    selected += [p.name for p in evidence.glob('*hour-final-reopen-*-input.png')]
    selected += [p.name for p in evidence.glob('*workspace-overview-*.png')]
    selected += [p.name for p in evidence.glob('*resume-failure.png')]
    files = {name: sha(evidence / name) for name in selected}
    result['contexts'][cohort] = {'run': run, 'core_input_hour_passed': summary['core_input_hour_passed'],
        'native_session_failure_observed': summary['native_session_failure_observed'],
        'actual_core_elapsed_sec': summary['elapsed'], 'core_checkpoints': summary['core_checkpoints'],
        'byte_exact_saves_including_preflight': summary['core_byte_exact'],
        'normal_closes_including_preflight': summary['normal_closes'],
        'source_stage_count': len(stage_sources), 'artifacts': artifacts,
        'selected_evidence_sha256': files, 'summary_sha256': sha(ROOT / (cohort + '-summary.json'))}
result['analysis_sha256'] = {name: sha(ROOT / name) for name in ['summarize_soak.py', 'summarize_round.py', 'r3-run-metadata.json', 'native-crash-diagnosis.json']}
result['report_sha256'] = sha(ROOT.parent / 'window-lockup-soak-r3-report.md')
(ROOT / 'r3-evidence-manifest.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({c: {k: v for k, v in d.items() if k not in ['artifacts', 'selected_evidence_sha256']}
                  for c, d in result['contexts'].items()}, indent=2))

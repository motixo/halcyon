"""Mandatory observable native acceptance. IPC alone cannot satisfy a scenario."""
SCENARIOS = {
    'clean-login': ('capture', 'processes', 'input'),
    'applets-input': ('capture', 'input', 'assertions'),
    'notifications': ('capture', 'notifications', 'assertions'),
    'one-host': ('processes', 'assertions'),
    'logout-login': ('login', 'processes', 'capture'),
    'crash-recovery': ('processes', 'notifications', 'capture'),
    'stock-fallback': ('processes', 'notifications', 'capture'),
    'hung-startup': ('processes', 'assertions', 'capture'),
    'responsive-ipc-no-bars': ('capture', 'assertions'),
    'notification-descriptor-failure': ('notifications', 'processes', 'assertions'),
    'reset-undo': ('config', 'capture', 'assertions'),
    'legacy-repair-confirmation': ('input', 'config', 'capture'),
    'light-dark': ('config', 'capture', 'portal'),
    'native-auto': ('clock', 'config', 'portal', 'capture'),
    'materials-four': ('capture', 'config', 'input'),
    'radii-small-large': ('capture', 'config', 'input'),
    'vertical-bars': ('capture', 'config', 'input'),
    'transparent-gaps': ('capture', 'input', 'assertions'),
    'partial-packages': ('packages', 'processes', 'assertions'),
    'installed-new-running-old': ('packages', 'processes', 'assertions'),
    'actual-020-migration': ('packages', 'login', 'config', 'processes', 'capture'),
    'disable-after-generations': ('login', 'config', 'processes', 'capture'),
    'disabled-new-external-users': ('config', 'assertions'),
}
OPTIONAL = {'fractional-scale': ('capture','input'), 'multi-monitor': ('capture','input'),
            'physical-gpu': ('graphics','capture','input')}


# Group4 supplies these observations from the integrated final main+Islands run.
SYSTEM_UPDATES = {
    'updates-discovery': ('packages', 'input', 'capture', 'assertions'),
    'updates-authorization-cancel': ('packages', 'input', 'config', 'assertions'),
    'updates-apply-relogin': ('packages', 'login', 'processes', 'capture', 'assertions'),
    'updates-failure-recovery': ('packages', 'config', 'capture', 'assertions'),
}


def validate(scenarios, artifacts, required=None):
    reasons = []
    if not isinstance(scenarios, dict):
        return ['scenario_results_missing']
    for name, kinds in (SCENARIOS if required is None else required).items():
        row = scenarios.get(name, {})
        if row.get('result') != 'pass' or row.get('native') is not True:
            reasons.append('scenario_incomplete:' + name)
        evidence = row.get('evidence', {})
        for kind in kinds:
            refs = evidence.get(kind, [])
            if not refs or not all(ref in artifacts and artifacts[ref]['kind'] == kind for ref in refs):
                reasons.append('observation_missing:' + name + ':' + kind)
        if not row.get('assertions') or not all(a.get('passed') is True and a.get('observed') is not None and
                                                a.get('expected') is not None for a in row.get('assertions', [])):
            reasons.append('assertions_missing:' + name)
    return reasons

#!/usr/bin/env python3
"""Native follow-mode scenario for stock/companion bars, with Halcyon stopped.

Inputs are real planner exports from the ignored Rust test
`desktop::profiles::themes::color_tests::native_theme_mode_fixtures` (set
HALCYON_MODE_EVIDENCE to an absolute private output directory). --harness names
an isolated native Session module: start/launch/screen/stop, fixture/logs/env.
The existing retained native.py provides this interface. A complete-session
qualifier can provide its own isolated Session implementation.

Halcyon never runs, and no user's bus or service manager is contacted. This scenario is
nested/render development evidence unless run by a complete-session qualifier.
--legacy-fixed reconstructs the 0.2.0 fixed-background contract using the same
preset seed's original variant; it is not tagged-binary/login qualification.
"""
import argparse
from collections import Counter
import importlib.util
import json
from pathlib import Path
import shutil
import os
import signal
import subprocess
import time


def put(root, namespace, version, key, value):
    path = root / namespace / ('v' + str(version)) / key
    path.parent.mkdir(parents=True, exist_ok=True)
    pending = path.with_name('.mode-scenario-' + key)
    pending.write_text(value)
    pending.replace(path)


def raw_keys(root):
    result = {}
    for path in root.rglob('*'):
        if path.is_file():
            result[str(path.relative_to(root))] = path.read_bytes().hex()
    return result


def pixel_evidence(path, expected):
    # ImageMagick is already supplied by the native harness environment.
    w, h = map(int, subprocess.check_output(['identify', '-format', '%w %h', str(path)], timeout=10).split())
    result = {}
    for bar, bounds in [('panel', (w//4, 0, w*3//4, 90)),
                        ('dock', (w//4, h-150, w*3//4, h))]:
        x, y, right, bottom = bounds
        data = subprocess.check_output(['convert', str(path), '-crop', f'{right-x}x{bottom-y}+{x}+{y}', '-depth', '8', 'rgb:-'], timeout=10)
        pixels = list(zip(data[0::3], data[1::3], data[2::3]))
        width = right-x
        def near(rgb, target, tolerance):
            return sum((a-b)**2 for a, b in zip(rgb, target)) < tolerance**2
        surface = [(i % width, i // width) for i, rgb in enumerate(pixels)
                   if near(rgb, expected['background'], 12)]
        foreground = 0
        if surface:
            left_edge = min(x for x, y in surface)
            right_edge = max(x for x, y in surface)
            top = min(y for x, y in surface)
            bottom = max(y for x, y in surface)
            # Measure text only inside the surface; desktop pixels beyond a
            # rounded bar must never count as dark foreground on a light bar.
            inset = (bottom-top)//2
            foreground = sum(1 for i, rgb in enumerate(pixels)
                             if left_edge+inset <= i % width <= right_edge-inset
                             and top+4 <= i // width <= bottom-4
                             and near(rgb, expected['foreground'], 35))
        result[bar] = {'role_pixels': {'background': len(surface), 'foreground': foreground},
                       'dominant': Counter(pixels).most_common(4),
                       'visible': len(surface) > 200 and foreground > 12}

    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--harness', type=Path, required=True)
    parser.add_argument('--fixtures', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--stock', type=Path, default=Path('/usr/bin/cosmic-panel'))
    parser.add_argument('--companion', type=Path)
    parser.add_argument('--legacy-fixed', action='store_true')
    parser.add_argument('--settings-daemon', action='store_true', help='launch native daemon only with a complete private session bus environment')
    parser.add_argument('--look', choices=['halcyon', 'calm', 'evening', 'focus', 'paper', 'rose'], action='append')
    args = parser.parse_args()
    args.output = args.output.resolve()
    if args.output.exists():
        parser.error('choose a new output directory; evidence is never overwritten')
    spec = importlib.util.spec_from_file_location('isolated_native', args.harness.resolve())
    native = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(native)
    hosts = [('stock', args.stock.resolve())]
    if args.companion:
        hosts.append(('companion', args.companion.resolve()))
    looks = args.look or ['halcyon', 'calm', 'evening', 'focus', 'paper', 'rose']
    args.output.mkdir(parents=True)
    summary = {'scope': 'isolated native rendering; no OS/login qualification',
               'legacy_fixed_reconstruction': args.legacy_fixed, 'halcyon_running': False, 'cases': []}
    failed = False
    for host, binary in hosts:
        for look in looks:
            run = args.output / (host + '-' + look)
            session = native.Session(run)
            root = session.fixture / 'config/cosmic'
            for namespace in ['com.system76.CosmicPanel', 'com.system76.CosmicPanel.Dock',
                              'com.system76.CosmicPanel.Panel', 'com.system76.CosmicTheme.Dark',
                              'com.system76.CosmicTheme.Light']:
                installed = Path('/usr/share/cosmic') / namespace
                if installed.exists():
                    shutil.copytree(installed, root / namespace, dirs_exist_ok=True)
            fixture = args.fixtures.resolve() / look
            shutil.copytree(fixture / 'cosmic', root, dirs_exist_ok=True)
            expected = json.loads((fixture / 'expected.json').read_text())
            put(root, 'com.system76.CosmicPanel', 1, 'entries', '["Panel","Dock"]')
            for bar in ['Panel', 'Dock']:
                ns = 'com.system76.CosmicPanel.' + bar
                put(root, ns, 1, 'plugins_wings', 'None')
                put(root, ns, 1, 'plugins_center', 'Some(["com.system76.CosmicAppletTime"])')
                put(root, ns, 1, 'autohide', 'None')
                # Opaque surfaces make role measurements unambiguous. The
                # generated family colors, corners and layout are unchanged.
                put(root, ns, 1, 'opacity', '1.0')
                if args.legacy_fixed:
                    rgb = expected['light' if look == 'paper' else 'dark']['background']
                    put(root, ns, 1, 'background', 'Color((' + ','.join(str(v/255) for v in rgb) + '))')
            put(root, 'com.system76.CosmicTheme.Mode', 1, 'is_dark', 'true')
            put(root, 'com.system76.CosmicTheme.Mode', 1, 'auto_switch', 'false')
            case = {'host': host, 'look': look, 'binary': str(binary), 'frames': []}
            try:
                session.start()
                # Explicit evidence that every process uses the private paths.
                for key in ['HOME', 'XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_STATE_HOME']:
                    assert Path(session.env[key]).is_relative_to(session.fixture), (key, session.env[key])
                assert session.env.get('DBUS_SESSION_BUS_ADDRESS')
                if args.settings_daemon:
                    session.launch('settings-daemon', ['/usr/bin/cosmic-settings-daemon'])
                    time.sleep(1)
                panel = session.launch('panel', [str(binary)])
                time.sleep(2)
                before = raw_keys(root)
                for automatic, dark in [(False, True), (False, False), (False, True),
                                        (True, False), (True, True)]:
                    put(root, 'com.system76.CosmicTheme.Mode', 1, 'auto_switch', str(automatic).lower())
                    put(root, 'com.system76.CosmicTheme.Mode', 1, 'is_dark', str(dark).lower())
                    time.sleep(.7)
                    label = ('auto-' if automatic else 'manual-') + ('dark' if dark else 'light') + '-' + str(len(case['frames']))
                    image = run / (label + '.png')
                    try:
                        session.screen(image)
                    except ModuleNotFoundError as error:
                        if error.name != "PIL":
                            raise
                        # Retained harnesses may lack Pillow. Capture only the
                        # explicitly private nested X display with ImageMagick.
                        for dims in [("1602", "1002"), ("1600", "1000")]:
                            session.xdo("windowsize", "2097154", *dims)
                            time.sleep(.25)
                        time.sleep(.6)
                        subprocess.run(["import", "-display", session.env["DISPLAY"], "-window", "root", '-crop', '1600x1000+0+0', str(image)], env=session.env, timeout=10, check=True)
                    pixels = pixel_evidence(image, expected['dark' if dark else 'light'])
                    alive = panel.poll() is None
                    frame = {'mode': label, 'running': alive, 'pixels': pixels}
                    case['frames'].append(frame)
                    (run / (label + '-keys.json')).write_text(json.dumps(raw_keys(root), indent=2))
                    if not alive or not all(p['visible'] for p in pixels.values()):
                        failed = True
                after = raw_keys(root)
                unchanged = all(after.get(k) == v for k, v in before.items() if 'CosmicTheme.Mode/' not in k)
                case['non_mode_keys_unchanged'] = unchanged
                failed |= not unchanged
            except Exception as error:
                case['error'] = str(error)
                failed = True
            finally:
                owned = dict(session.pids)
                case['cleanup'] = session.stop()
                # Some session daemons do not exit on SIGTERM without their
                # full peers. Reap only process groups launched by this case.
                for name, state in case['cleanup'].items():
                    if state not in ('Z', 'exited') and name != 'dbus':
                        try:
                            os.killpg(owned[name], signal.SIGKILL)
                            case['cleanup'][name] = 'killed after SIGTERM timeout'
                        except ProcessLookupError:
                            case['cleanup'][name] = 'exited'
                summary['cases'].append(case)
                (args.output / 'results.json').write_text(json.dumps(summary, indent=2) + '\n')
                print(host, look, json.dumps(case), flush=True)
    # Legacy reconstruction is expected to reveal failures; report the raw
    # observations without presenting it as a passing current implementation.
    summary['all_frames_mode_coherent'] = not failed
    (args.output / 'results.json').write_text(json.dumps(summary, indent=2) + '\n')
    return 1 if failed and not args.legacy_fixed else 0

if __name__ == '__main__':
    raise SystemExit(main())

#!/usr/bin/env python3
"""Stage the reviewed panel for the next login. Never restart a live session."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
from panel_launcher import digest, session_identity


def sync_directory(path):
    directory = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)


def place(path, data, mode):
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temp = tempfile.mkstemp(prefix='.' + path.name + '.', dir=path.parent)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(data)
            stream.flush()
            os.fchmod(stream.fileno(), mode)
            os.fsync(stream.fileno())
        os.replace(temp, path)
        sync_directory(path.parent)
    finally:
        Path(temp).unlink(missing_ok=True)


def safe_path(root, relative):
    path = root / relative
    if Path(relative).is_absolute() or '..' in Path(relative).parts:
        raise ValueError('Invalid companion install path')
    for part in [path, *path.parents]:
        if part == root:
            break
        if part.is_symlink():
            raise ValueError('A symlink occupies a companion install path')
    return path


def install(root, binary, manifest, enable=False, defer_session=None):
    root, binary, manifest = root.resolve(), binary.resolve(), manifest.resolve()
    record = json.loads(manifest.read_text())
    if digest(binary) != record['binary_sha256']:
        raise ValueError('Companion binary does not match the reviewed manifest')
    selection = {'enabled': enable}
    if enable and defer_session is not None:
        selection['defer_session'] = defer_session
    prefix = '.local/share/cosmic-desktop-cards/companion/'
    contents = {
        prefix + 'cosmic-panel-islands': (binary.read_bytes(), 0o755),
        prefix + 'manifest.json': (manifest.read_bytes(), 0o644),
        '.local/bin/cosmic-panel': (Path(__file__).with_name('panel_launcher.py').read_bytes(), 0o755),
        prefix + 'selection.json': ((json.dumps(selection) + '\n').encode(), 0o600),
    }
    paths = {rel: safe_path(root, rel) for rel in contents}
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
    backup = safe_path(root, '.local/share/cosmic-desktop-cards/backups/panel-' + stamp)
    backup.mkdir(parents=True, mode=0o700)
    before, originals = {}, {}
    for rel, path in paths.items():
        if path.exists():
            destination = backup / path.name
            shutil.copy2(path, destination)
            with destination.open('rb') as stream:
                os.fsync(stream.fileno())
            before[rel] = str(destination.relative_to(root))
            originals[rel] = {'sha256': digest(destination), 'mode': destination.stat().st_mode & 0o777}
    result = {
        'root': str(root), 'backup': str(backup), 'before': before, 'originals': originals,
        'installed': {rel: hashlib.sha256(data).hexdigest() for rel, (data, _) in contents.items()},
        'modes': {rel: mode for rel, (_, mode) in contents.items()},
        'complete': False, 'restored': False,
    }
    record_path = backup / 'install-record.json'
    # Record every intended replacement before the first mutation. This also
    # permits safe recovery after a hard interruption between replace and return.
    place(record_path, (json.dumps(result, indent=2) + '\n').encode(), 0o600)
    try:
        for rel, path in paths.items():
            data, mode = contents[rel]
            place(path, data, mode)
        result['complete'] = True
        place(record_path, (json.dumps(result, indent=2) + '\n').encode(), 0o600)
    except BaseException:
        restore(record_path)
        raise
    return result


def restore(record_path):
    record = json.loads(Path(record_path).read_text())
    root = Path(record['root'])
    if record.get('restored'):
        sync_directory(Path(record_path).parent)
        return
    restore_paths = []
    # Validate the entire rollback before removing the candidate or replacing
    # its selector. A missing/corrupt backup must leave the install intact.
    for rel, expected in record['installed'].items():
        path = safe_path(root, rel)
        current = {'sha256': digest(path), 'mode': path.stat().st_mode & 0o777} if path.is_file() else None
        old = record['originals'].get(rel)
        if rel in record['before']:
            original = safe_path(root, record['before'][rel])
            if (not original.is_file() or digest(original) != old['sha256']
                    or original.stat().st_mode & 0o777 != old['mode']):
                raise ValueError('Companion rollback backup checksum or mode failed: ' + rel)
        if path.exists() and not path.is_file():
            raise ValueError('A non-file occupies a companion install path: ' + rel)
        if current != old and current != {'sha256': expected, 'mode': record['modes'][rel]}:
            raise ValueError('A companion install file changed since installation: ' + rel)
        restore_paths.append(rel)
    # Each step accepts and durably replays its original state. Retrying also
    # finishes a replacement or removal whose directory sync was interrupted.
    for rel in restore_paths:
        path = safe_path(root, rel)
        if rel in record['before']:
            original = safe_path(root, record['before'][rel])
            place(path, original.read_bytes(), record['originals'][rel]['mode'])
        else:
            path.unlink(missing_ok=True)
            if path.parent.exists():
                sync_directory(path.parent)
    record['restored'] = True
    place(Path(record_path), (json.dumps(record, indent=2) + '\n').encode(), 0o600)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    target = parser.add_mutually_exclusive_group(required=True)
    target.add_argument('--root', type=Path)
    target.add_argument('--user', action='store_true')
    target.add_argument('--restore', type=Path)
    parser.add_argument('--binary', type=Path)
    parser.add_argument('--manifest', type=Path)
    parser.add_argument('--enable-next-login', action='store_true')
    args = parser.parse_args()
    if args.restore:
        restore(args.restore)
        return
    if not args.binary or not args.manifest:
        parser.error('--binary and --manifest are required')
    root = Path.home() if args.user else args.root
    if args.user:
        paths = os.environ.get('PATH', '').split(os.pathsep)
        if str(root / '.local/bin') not in paths or '/usr/bin' not in paths or paths.index(str(root / '.local/bin')) > paths.index('/usr/bin'):
            raise SystemExit('The session PATH must put ~/.local/bin before /usr/bin')
    defer_session = session_identity() if args.enable_next_login else None
    if args.enable_next_login and defer_session is None:
        raise SystemExit('Run from the COSMIC desktop session so selection can wait for the next login')
    result = install(root, args.binary, args.manifest, args.enable_next_login, defer_session)
    print(json.dumps(result, indent=2))
    print('Prepared for the next login. No running process or desktop preference was changed.', file=sys.stderr)


if __name__ == '__main__':
    main()

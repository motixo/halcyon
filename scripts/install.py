#!/usr/bin/env python3
"""Install or upgrade Desktop Cards for the current user.

Copies a release build, the controller, the data helpers, the source (with the
bundled wallpapers), the user service and the launcher into the home folder.
Preferences, notes, drafts, profiles, automation and undo records are never
touched, and nothing is enabled or started: `desktop-cards start` does that.
An upgrade keeps the previous binary and controller in a dated backup folder.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import sys
import tempfile

SOURCE = Path(__file__).resolve().parent.parent
SERVICE = 'cosmic-desktop-cards.service'
LAUNCHER = 'io.github.workstation.DesktopCards.desktop'
# Kept out of the installed source copy.
SKIP = shutil.ignore_patterns('target', '__pycache__', '.git', '*.pyc')

UNIT = """[Unit]
Description=Coordinated COSMIC desktop clock, weather and devbox cards
PartOf=graphical-session.target
After=graphical-session.target

[Service]
ExecStart=%h/.local/bin/cosmic-desktop-cards
Restart=on-failure
RestartSec=5

[Install]
WantedBy=graphical-session.target
"""


def launcher(home):
    return f"""[Desktop Entry]
Type=Application
Name=Desktop Cards
Comment=Desktop widgets, wallpapers and COSMIC customization
Exec={home}/.local/bin/desktop-cards settings
Icon=preferences-desktop-wallpaper
Terminal=false
Categories=Settings;DesktopSettings;
Keywords=widgets;weather;wallpaper;themes;clock;dock;panel;profiles;
StartupWMClass=io.github.workstation.DesktopCards.Settings
"""


def default_binary():
    target = Path(os.environ.get('CARGO_TARGET_DIR', SOURCE / 'target'))
    if not target.is_absolute():
        target = SOURCE / target
    return target / 'release' / 'cosmic-desktop-cards'


def digest(path):
    hasher = hashlib.sha256()
    with Path(path).open('rb') as file:
        for block in iter(lambda: file.read(1 << 20), b''):
            hasher.update(block)
    return hasher.hexdigest()


def same(path, data):
    try:
        return Path(path).read_bytes() == data
    except FileNotFoundError:
        return False


def place(path, data, mode):
    """Writes `data` atomically; returns whether anything changed."""
    path = Path(path)
    if same(path, data) and (path.stat().st_mode & 0o777) == mode:
        return False
    path.parent.mkdir(parents=True, exist_ok=True)
    handle, name = tempfile.mkstemp(dir=path.parent, prefix=f'.{path.name}.')
    try:
        with os.fdopen(handle, 'wb') as file:
            file.write(data)
            file.flush()
            os.fsync(file.fileno())
        os.chmod(name, mode)
        os.replace(name, path)
    finally:
        Path(name).unlink(missing_ok=True)
    return True


def replace_tree(source, destination):
    """Swaps in a fresh copy of the source; the old copy goes only after the swap."""
    destination = Path(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    fresh = Path(tempfile.mkdtemp(dir=destination.parent, prefix='.source-new-'))
    old = destination.with_name('.source-old')
    try:
        shutil.copytree(source, fresh, ignore=SKIP, dirs_exist_ok=True)
        shutil.rmtree(old, ignore_errors=True)
        if destination.exists():
            os.replace(destination, old)
        os.replace(fresh, destination)
    finally:
        shutil.rmtree(fresh, ignore_errors=True)
        shutil.rmtree(old, ignore_errors=True)


def google_client(path):
    """Validate publisher configuration before installing anything; never log it."""
    try:
        with Path(path).open('rb') as file:
            raw = file.read(64 * 1024 + 1)
        if len(raw) > 64 * 1024:
            raise ValueError()
        value = json.loads(raw)['installed']
        client_id, secret = value['client_id'], value['client_secret']
        if (not isinstance(client_id, str) or not isinstance(secret, str)
                or not client_id.endswith('.apps.googleusercontent.com')
                or len(client_id) > 512 or any(c.isspace() for c in client_id)
                or not 0 < len(secret) <= 512):
            raise ValueError()
        # Ignore imported endpoints and unrelated data. The app pins Google's endpoints.
        return json.dumps({'installed': {'client_id': client_id,
                                        'client_secret': secret}}).encode()
    except (KeyError, TypeError, ValueError, OSError):
        raise RuntimeError('Choose a valid Google OAuth JSON file for a Desktop app.') from None


def install(binary, home, *, systemd=True, restart=False, google_oauth_client=None,
            run=subprocess.run):
    binary, home = Path(binary), Path(home)
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise RuntimeError(f'No release build at {binary}. Run: cargo build --release --locked')
    client = google_client(google_oauth_client) if google_oauth_client is not None else None
    bin_dir = home / '.local/bin'
    data = home / '.local/share/cosmic-desktop-cards'
    app = bin_dir / 'cosmic-desktop-cards'
    controller = bin_dir / 'desktop-cards'
    first = not app.exists()
    new_app = binary.read_bytes()
    new_controller = (SOURCE / 'scripts/desktop-cards').read_bytes()

    # Keep what is being replaced, so an upgrade can be rolled back by hand.
    replaced = [path for path, new in ((app, new_app), (controller, new_controller))
                if path.exists() and not same(path, new)]
    backup = None
    if replaced:
        stamp = datetime.now(timezone.utc).strftime('%Y%m%d-%H%M%S')
        backup = data / 'backups' / f'{stamp}-install'
        backup.mkdir(parents=True, exist_ok=True)
        for path in replaced:
            shutil.copy2(path, backup / path.name)

    place(app, new_app, 0o755)
    place(controller, new_controller, 0o755)
    for helper in ('calendar-sync.py', 'project-status.py'):
        place(data / helper, (SOURCE / 'scripts' / helper).read_bytes(), 0o755)
    if client is not None:
        place(data / 'google-calendar-client.json', client, 0o600)
    replace_tree(SOURCE, data / 'source')
    unit_changed = place(home / '.config/systemd/user' / SERVICE, UNIT.encode(), 0o644)
    place(home / '.local/share/applications' / LAUNCHER, launcher(home).encode(), 0o644)
    wallpapers = runpy.run_path(str(data / 'source/scripts/install_wallpapers.py'))
    wallpapers['install_wallpapers'](home / 'Pictures/Desktop Cards')

    if systemd:
        if unit_changed:
            run(['systemctl', '--user', 'daemon-reload'], check=True)
        if restart:
            # Restarts only a running service; never enables or starts one.
            run(['systemctl', '--user', 'try-restart', SERVICE], check=True)
    return {'first': first, 'sha256': digest(app), 'backup': backup}


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--binary', type=Path, default=default_binary(),
                        help='release build to install (default: %(default)s)')
    parser.add_argument('--restart', action='store_true',
                        help='restart the cards if they are already running')
    parser.add_argument('--google-oauth-client', type=Path,
                        help='publisher Google Desktop app JSON (optional; preserves an existing client)')
    parser.add_argument('--no-systemd', action='store_true', help=argparse.SUPPRESS)
    parser.add_argument('--home', type=Path, default=Path.home(), help=argparse.SUPPRESS)
    args = parser.parse_args()
    result = install(args.binary, args.home, systemd=not args.no_systemd, restart=args.restart,
                     google_oauth_client=args.google_oauth_client)
    print(f"Installed cosmic-desktop-cards {result['sha256'][:12]} in {args.home / '.local/bin'}")
    if result['backup']:
        print(f"Previous version kept in {result['backup']}")
    if result['first']:
        print('Run `desktop-cards start` to show the cards and start them at login, '
              'or open Desktop Cards from the launcher.')
    elif not args.restart:
        print('Run `desktop-cards restart` (or log out and in) to use the new version.')


if __name__ == '__main__':
    try:
        main()
    except (RuntimeError, OSError) as exc:
        print(str(exc), file=sys.stderr)
        sys.exit(1)

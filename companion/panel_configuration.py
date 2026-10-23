"""Read-only launch inventory for the pinned native cosmic-panel config semantics.

This is deliberately a bounded supported subset, not a general RON/desktop-entry
interpreter. Unsupported/ambiguous input selects stock before launching. No user
configuration is written. See docs/COMPATIBILITY.md for the qualified scope.
"""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import socket
import struct
import time
from collections import deque

from bundle_contract import identity

RESOLVER = 'cosmic-panel-ca20ce3-native-v1'
PREFIX = 'com.system76.CosmicPanel'
ID = re.compile(r'[A-Za-z0-9_.-]{1,200}')


class Changed(ValueError):
    pass


class Watch:
    """Watch dependencies before reading, including absence and replacements/ABA."""
    def __init__(self):
        self.lib = ctypes.CDLL(None, use_errno=True)
        self.fd = self.lib.inotify_init1(os.O_NONBLOCK | os.O_CLOEXEC)
        if self.fd < 0:
            raise OSError(ctypes.get_errno(), 'inotify unavailable')
        self.paths = {}
        self.dirty = False

    def add(self, path, directory=False):
        path = Path(path)
        # Following arbitrary symlink config trees would require also watching
        # every link target/ancestor. Decline that scope rather than guessing.
        if any(p.is_symlink() for p in (path, *path.parents)):
            raise ValueError('configuration_symlink_unsupported')
        parent, name = (path, None) if directory and path.is_dir() else (path.parent, path.name)
        while not parent.is_dir():
            name, parent = parent.name, parent.parent
        mask = 0x2 | 0x4 | 0x8 | 0x40 | 0x80 | 0x100 | 0x200 | 0x400 | 0x800
        wd = self.lib.inotify_add_watch(self.fd, os.fsencode(parent), mask)
        if wd < 0:
            raise OSError(ctypes.get_errno(), 'configuration watch failed')
        self.paths.setdefault(wd, set()).add(name)
        if len(self.paths) > 4096:
            raise ValueError('configuration_watch_limit')

    def changed(self):
        while self.fd >= 0:
            try:
                data = os.read(self.fd, 65536)
            except BlockingIOError:
                break
            if not data:
                break
            offset = 0
            while offset < len(data):
                wd, mask, _, size = struct.unpack_from('iIII', data, offset)
                name = os.fsdecode(data[offset+16:offset+16+size].split(b'\0')[0])
                watched = self.paths.get(wd, set())
                if mask & (0x4000 | 0x8000 | 0x400 | 0x800) or None in watched or name in watched:
                    self.dirty = True
                offset += 16 + size
        return self.dirty

    def close(self):
        if self.fd >= 0:
            os.close(self.fd)
            self.fd = -1

    def __del__(self):
        self.close()


# Serialized COSMIC lists/options/tuples/strings and output variants, including
# whitespace, trailing commas and nested comments. Other RON forms fail closed.
def ron(text):
    tokens = []
    i = 0
    while i < len(text):
        if text[i].isspace(): i += 1; continue
        if text.startswith('//', i):
            end = text.find('\n', i); i = len(text) if end < 0 else end+1; continue
        if text.startswith('/*', i):
            depth = 1; i += 2
            while i < len(text) and depth:
                if text.startswith('/*', i): depth += 1; i += 2
                elif text.startswith('*/', i): depth -= 1; i += 2
                else: i += 1
            if depth: raise ValueError('ron_comment')
            continue
        if text[i] == '"':
            # COSMIC identifiers/paths are the supported ASCII subset. Reject
            # unsupported escape syntax instead of changing its interpretation.
            value, length = json.JSONDecoder().raw_decode(text[i:])
            if any(escape not in ('\\', '"', 'n', 'r', 't') for escape in re.findall(r'\\(.)', text[i:i+length])):
                raise ValueError('ron_string_escape_unsupported')
            tokens.append(('string', value)); i += length; continue
        match = re.match(r'[A-Za-z_][A-Za-z_0-9]*|[\[\](),]', text[i:])
        if not match: raise ValueError('ron_unsupported')
        tokens.append(match[0]); i += len(match[0])
    pos = 0
    def value(depth=0):
        nonlocal pos
        if depth > 12 or pos >= len(tokens): raise ValueError('ron_depth_or_end')
        token = tokens[pos]; pos += 1
        if isinstance(token, tuple): return token[1]
        if token in ('[', '('):
            end = ']' if token == '[' else ')'; items = []
            while pos < len(tokens) and tokens[pos] != end:
                items.append(value(depth+1))
                if pos < len(tokens) and tokens[pos] == ',': pos += 1
                elif pos >= len(tokens) or tokens[pos] != end: raise ValueError('ron_comma')
            if pos >= len(tokens): raise ValueError('ron_end')
            pos += 1
            return items if token == '[' else tuple(items)
        if token in ('Some', 'Name'):
            arg = value(depth+1)
            if not isinstance(arg, tuple) or len(arg) != 1: raise ValueError('ron_variant')
            return ('variant', token, arg[0])
        if token in ('None', 'All', 'Active'): return ('variant', token)
        raise ValueError('ron_variant')
    result = value()
    if pos != len(tokens): raise ValueError('ron_trailing')
    return result


def strings(value):
    if not isinstance(value, list) or len(value) > 64 or not all(isinstance(v, str) and ID.fullmatch(v) for v in value):
        raise ValueError('configuration_identifiers')
    return value


def optional(value):
    if value == ('variant', 'None'): return None
    if isinstance(value, tuple) and len(value) == 3 and value[:2] == ('variant', 'Some'): return value[2]
    raise ValueError('configuration_option')


def output_names(env=None):
    """Read wl_output v4 names through a separate bounded registry connection.

    Never consumes WAYLAND_SOCKET, launches a host, or guesses DRM connectors.
    The second sync completes initial output events. Removal/old protocol fails
    closed. Native qualification must verify this protocol/IO scope.
    """
    env = os.environ if env is None else env
    display = env.get('WAYLAND_DISPLAY')
    if not display: raise ValueError('wayland_display_missing')
    path = Path(display) if display.startswith('/') else Path(env['XDG_RUNTIME_DIR']) / display
    deadline = time.monotonic() + 1
    def string(s):
        b = s.encode()+b'\0'; return struct.pack('I', len(b))+b+b'\0'*((-len(b)) % 4)
    def decode(b):
        if len(b) < 4: raise ValueError('wayland_string')
        size, = struct.unpack_from('I', b)
        if not 1 <= size <= len(b)-4 or b[3+size] != 0: raise ValueError('wayland_string')
        return b[4:3+size].decode(), 4+((size+3)//4)*4
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.settimeout(1); client.connect(str(path))
        def send(obj, opcode, payload): client.sendall(struct.pack('II', obj, ((len(payload)+8)<<16)|opcode)+payload)
        def receive(n):
            result = b''
            while len(result) < n:
                left = deadline-time.monotonic()
                if left <= 0: raise ValueError('wayland_deadline')
                client.settimeout(left); chunk = client.recv(n-len(result))
                if not chunk: raise ValueError('wayland_closed')
                result += chunk
            return result
        send(1, 1, struct.pack('I', 2)); send(1, 0, struct.pack('I', 3))
        outputs = {}; next_id = 10; phase = 3
        for _ in range(4096):
            obj, header = struct.unpack('II', receive(8)); size, op = header >> 16, header & 65535
            if not 8 <= size <= 65536 or size % 4: raise ValueError('wayland_message')
            body = receive(size-8)
            if obj == 1 and op == 0: raise ValueError('wayland_error')
            if obj == 2:
                if op == 1: raise Changed('wayland_outputs_changed')
                if op != 0 or len(body) < 12: raise ValueError('wayland_registry')
                global_id, = struct.unpack_from('I', body); interface, off = decode(body[4:])
                if len(body) != off+8: raise ValueError('wayland_registry_size')
                version, = struct.unpack_from('I', body, off+4)
                if interface == 'wl_output':
                    if phase != 3 or version < 4 or len(outputs) >= 32: raise ValueError('wayland_output_scope')
                    outputs[next_id] = None
                    send(2, 0, struct.pack('I', global_id)+string('wl_output')+struct.pack('II', 4, next_id)); next_id += 1
            elif obj in outputs and op == 4:
                if outputs[obj] is not None: raise Changed('wayland_name_changed')
                outputs[obj] = decode(body)[0]
            elif obj == phase and op == 0:
                if phase == 3:
                    phase = 4; send(1, 0, struct.pack('I', 4))
                else:
                    names = list(outputs.values())
                    if not names or any(not isinstance(n,str) or not ID.fullmatch(n) for n in names) or len(set(names)) != len(names):
                        raise ValueError('wayland_output_names')
                    return sorted(names)
    raise ValueError('wayland_event_limit')


class Snapshot:
    def __init__(self, policy, env=None):
        self.env = dict(os.environ if env is None else env)
        self.watch = Watch(); self.records = {}; self.policy = policy
        try:
            self.outputs = output_names(self.env)
            if len(self.outputs) > policy['max_outputs']: raise ValueError('unqualified_output_count')
            self.counts, bars = self._inventory()
            self.identity = identity(dict(resolver=RESOLVER, outputs=self.outputs, bars=bars, files=self.records,
                                          counts=self.counts, path=self.env.get('PATH')))
            if self.watch.changed() or output_names(self.env) != self.outputs: raise Changed('configuration_changed')
        except BaseException:
            self.close(); raise

    def close(self): self.watch.close()
    def changed(self):
        if self.watch.changed(): return True
        try: return output_names(self.env) != self.outputs
        except (OSError, ValueError, KeyError): return True

    def read(self, path):
        path = Path(path); self.watch.add(path)
        if not path.exists(): self.records[str(path)] = None; return None
        if not path.is_file() or path.stat().st_size > 65536: raise ValueError('configuration_file')
        data = path.read_bytes(); self.records[str(path)] = hashlib.sha256(data).hexdigest()
        return data.decode()

    def directory(self, path):
        self.watch.add(path, directory=True)
        if not path.exists(): self.records[str(path)] = None; return False
        if not path.is_dir(): raise ValueError('configuration_directory')
        return True

    def _inventory(self):
        env = self.env
        if env.get('FLATPAK_ID') or env.get('COSMIC_PANEL_CONFIG'): raise ValueError('configuration_environment_unsupported')
        home = Path(env['HOME'])
        config_home = Path(env.get('XDG_CONFIG_HOME') or home/'.config')
        data_home = Path(env.get('XDG_DATA_HOME') or home/'.local/share')
        data_dirs = [data_home]+[Path(p) for p in (env.get('XDG_DATA_DIRS') or '/usr/local/share:/usr/share').split(':') if p.startswith('/')]
        if not home.is_absolute() or not config_home.is_absolute() or not data_home.is_absolute(): raise ValueError('configuration_relative_root')
        system = {}
        def get(name, key, default):
            relative = Path('cosmic')/name/'v1'
            # cosmic-config selects the first existing namespace directory,
            # then falls back per-key only from the user config to that directory.
            if name not in system:
                system[name] = None
                for base in data_dirs:
                    if self.directory(base/relative): system[name] = base/relative; break
            raw = self.read(config_home/relative/key)
            if raw is None and system[name] is not None: raw = self.read(system[name]/key)
            return default if raw is None else ron(raw)
        entries = get(PREFIX, 'entries', None)
        if entries is None:
            # Container default applies only when entries itself is missing.
            # Per-entry missing fields otherwise use CosmicPanelConfig::default.
            builtins = [('Panel', ['com.system76.CosmicPanelWorkspacesButton','com.system76.CosmicPanelAppButton',
                'com.system76.CosmicAppletTime','com.system76.CosmicAppletInputSources','com.system76.CosmicAppletA11y',
                'com.system76.CosmicAppletStatusArea','com.system76.CosmicAppletTiling','com.system76.CosmicAppletAudio',
                'com.system76.CosmicAppletBluetooth','com.system76.CosmicAppletNetwork','com.system76.CosmicAppletBattery',
                'com.system76.CosmicAppletNotifications','com.system76.CosmicAppletPower']),
                ('Dock',['com.system76.CosmicPanelLauncherButton','com.system76.CosmicPanelWorkspacesButton',
                 'com.system76.CosmicPanelAppButton','com.system76.CosmicAppList','com.system76.CosmicAppletMinimize'])]
            bars = [(name, ('variant','All'), plugins) for name,plugins in builtins]
        else:
            entries = strings(entries)
            if len(set(entries)) != len(entries): raise ValueError('duplicate_bar')
            bars = []
            for entry in entries:
                namespace = PREFIX+'.'+entry
                name = get(namespace,'name','')
                if name != entry: raise ValueError('ambiguous_bar_name')
                output = get(namespace,'output',('variant','All'))
                wings = optional(get(namespace,'plugins_wings',('variant','None')))
                center = optional(get(namespace,'plugins_center',('variant','None')))
                if wings is not None and (not isinstance(wings,tuple) or len(wings)!=2): raise ValueError('configuration_wings')
                plugins = (strings(wings[0]) if wings else []) + (strings(center) if center is not None else []) + (strings(wings[1]) if wings else [])
                bars.append((entry, output, plugins))
        counts = {p:0 for p in self.policy['processes']}
        wanted = {p for _,_,ps in bars for p in ps}
        if wanted - self.policy['applets'].keys(): raise ValueError('unqualified_configured_applet')
        # Pinned Iter visits direct files sorted, then queued subdirectories in
        # reverse encountered order, before the next XDG root. file_stem (not
        # flattened desktop ID) selects each plugin's first matching path.
        paths = {}; pending = deque(base/'applications' for base in data_dirs); seen = set(); visited = 0
        while pending and wanted - paths.keys():
            directory = pending.popleft()
            if directory in seen: continue
            seen.add(directory)
            if not self.directory(directory): continue
            for path in sorted(directory.iterdir()):
                visited += 1
                if visited > 10000: raise ValueError('desktop_inventory_limit')
                if path.is_dir(): pending.appendleft(path)
                elif path.suffix == '.desktop' and path.stem in wanted and path.stem not in paths: paths[path.stem] = path
        if wanted - paths.keys(): raise ValueError('configured_desktop_missing')
        for plugin,path in paths.items():
            fields = desktop(self.read(path))
            command = fields.get('Exec')
            if not command or 'X-Flatpak' in fields: raise ValueError('desktop_exec_unsupported')
            if any(c in command for c in ('#', '\\', '\n', '\r', '\0')):
                raise ValueError('desktop_exec_syntax_unsupported')
            argv = shlex.split(command, comments=False, posix=True)
            qualified = self.policy['applets'][plugin]
            if not argv or argv[1:] != qualified['args']: raise ValueError('desktop_arguments_unqualified')
            exe = Path(argv[0])
            if not exe.is_absolute():
                if '/' in argv[0]: raise ValueError('desktop_relative_exec')
                choices = [Path(p)/argv[0] for p in env.get('PATH',os.defpath).split(':')]
                if any(not p.is_absolute() for p in choices): raise ValueError('desktop_relative_path')
                exe = None
                for choice in choices:
                    if choice.is_file() and os.access(choice, os.X_OK): exe = choice; break
                    self.watch.add(choice)
                if exe is None: raise ValueError('desktop_exec_missing')
            # Executable bytes are verified by installed/runtime IO, not this
            # config watcher. Replacing code remains a code failure, not a race
            # excuse suppressing the crash latch.
            if str(exe.resolve()) != self.policy['processes'][qualified['process']]: raise ValueError('desktop_exec_unqualified')
        normalized = []
        for name,output,plugins in bars:
            if len(set(plugins)) != len(plugins): raise ValueError('duplicate_plugin_on_bar_unsupported')
            if output == ('variant','All'): multiplier = len(self.outputs)
            elif output == ('variant','Active'): multiplier = 1
            elif isinstance(output,tuple) and len(output)==3 and output[:2]==('variant','Name') and isinstance(output[2],str): multiplier = int(output[2] in self.outputs)
            else: raise ValueError('configuration_output')
            for plugin in plugins:
                process = self.policy['applets'][plugin]['process']; counts[process] += multiplier
                if counts[process] > 64: raise ValueError('configured_count_limit')
            normalized.append(dict(name=name,output=output,plugins=plugins,multiplier=multiplier))
        return counts, normalized


def desktop(text):
    if text is None: raise ValueError('desktop_missing')
    groups = {}; group = None
    for line in text.splitlines():
        if not line.strip() or line.startswith('#'): continue
        if line.startswith('[') and line.endswith(']'):
            group = line[1:-1]
            if group in groups: raise ValueError('desktop_duplicate_group')
            groups[group] = {}; continue
        if group is None or '=' not in line: raise ValueError('desktop_line')
        key,value = line.split('=',1); key=key.strip()
        if not key: raise ValueError('desktop_key')
        # Reject duplicate keys and localized Exec (not a portable launch
        # contract); the supported direct Exec is unambiguous across locales.
        if key in groups[group] or key.startswith('Exec['): raise ValueError('desktop_ambiguous_key')
        escapes={'s':' ','n':'\n','t':'\t','r':'\r','\\':'\\'}
        def unescape(match):
            if match[1] not in escapes: raise ValueError('desktop_escape')
            return escapes[match[1]]
        if value.endswith('\\') and len(value)-len(value.rstrip('\\')) & 1: raise ValueError('desktop_trailing_escape')
        if value.startswith(' '): value=value[1:]
        groups[group][key] = re.sub(r'\\(.)',unescape,value)
    if 'Desktop Entry' not in groups: raise ValueError('desktop_group')
    return groups['Desktop Entry']


def policy(profile):
    config = profile['configuration']
    return {**config, 'processes': {n:p['executable'] for n,p in profile['processes'].items() if p['lifecycle']=='panel-child'}}

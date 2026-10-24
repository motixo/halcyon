"""Constrain a native Settings exported ThemeBuilder; never generate a palette."""
import re
FLAGS = ('frosted_windows', 'frosted_system_interface', 'frosted_applets')

def fields(text):
    result = {}
    for name in (*FLAGS, 'frosted_panel', 'frosted'):
        hits = re.findall(r'^\s*' + name + r':\s*([^,\n]+),\s*$', text, re.M)
        if len(hits) != 1:
            raise ValueError('Native export has missing/ambiguous field: ' + name)
        result[name] = hits[0]
    if result['frosted'] != 'Medium' or result['frosted_panel'] != 'false':
        raise ValueError('Export differs from pinned R3 strength/panel treatment')
    if not re.search(r'^\s*palette:\s*Dark\(', text, re.M):
        raise ValueError('Comparison requires actual selected Dark builder')
    return result

def change_flags(text, enabled):
    before = fields(text)
    desired = 'true' if enabled else 'false'
    changed = text
    for name in FLAGS:
        changed, count = re.subn(r'^(\s*' + name + r':\s*)(?:true|false)(,\s*)$',
                                 lambda m: m[1] + desired + m[2], changed, flags=re.M)
        if count != 1:
            raise ValueError('Refuse uncontrolled native theme edit: ' + name)
    after = fields(changed)
    assert all(after[name] == desired for name in FLAGS)
    return changed, {'before': before, 'after': after,
                     'only_modified_builder_fields': list(FLAGS),
                     'native_writer': 'cosmic-settings appearance import'}

def verify_media(blocks, public):
    inserted = [b['inserted'] for b in blocks if b.get('removable') and b.get('inserted')]
    if public:
        if len(inserted)!=1 or inserted[0].get('ro') is not True or not inserted[0].get('file','').endswith('/candidate.iso'):
            raise ValueError('Public comparison requires exactly one identified read-only candidate ISO')
    elif inserted:
        raise ValueError('Untouched stock comparison must have no inserted candidate media')

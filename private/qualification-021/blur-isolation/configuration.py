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

def validate_reexport(requested, actual, stored_text_tint, generated_background):
    """Validate the native export projection against real generated native state.

    Settings ThemeCustomizer::from replaces builder text_tint with the built
    theme background.on color. This is not a round-trip serializer. Permit
    exactly that source-defined projection, never an expected palette fixture.
    """
    def tint(text):
        found=re.findall(r'^\s*text_tint:\s*([^\n]+),\s*$',text,re.M)
        if len(found)!=1:raise ValueError('Ambiguous native text_tint export')
        return found[0]
    if stored_text_tint.strip()!=tint(requested):
        raise ValueError('Stored native builder did not retain requested text_tint')
    if requested==actual:
        return {'kind':'byte-exact','stored_builder_text_tint':stored_text_tint.strip()}
    outer_on=re.findall(r'^    on:\s*("#[0-9A-Fa-f]{8}"),\s*$',generated_background,re.M)
    if len(outer_on)!=1:raise ValueError('Ambiguous actual generated background.on')
    projected=re.sub(r'^(\s*text_tint:\s*)([^\n]+)(,\s*)$',
                     lambda m:m[1]+'Some('+outer_on[0]+')'+m[3],requested,flags=re.M)
    if projected!=actual:
        raise ValueError('Native export changed beyond the source-defined foreground projection')
    return {'kind':'native-derived-foreground','stored_builder_text_tint':stored_text_tint.strip(),
            'generated_background_on':outer_on[0],'exported_text_tint':tint(actual),
            'source':'cosmic-settings f1d00fc theme_manager.rs ThemeCustomizer::from lines67-77'}

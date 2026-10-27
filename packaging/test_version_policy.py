"""Real Debian controls and isolated APT simulations; no package installation."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

import version_policy as policy

MAIN = 'cosmic-ext-halcyon'
ISLANDS = MAIN + '-islands'


class Pairing(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        (self.root/'packaging/islands').mkdir(parents=True)
        self.pair('0.5.0', '0.5.0')

    def pair(self, main, islands):
        (self.root/'Cargo.toml').write_text(f'[package]\nversion="{main}"\n')
        (self.root/'packaging/islands/download.json').write_text(json.dumps(dict(
            format='halcyon-islands-download',version=1,main_version=main,package_version=islands)))

    def package(self, name, version, relationships):
        tree = self.root/f'{name}-{version}'
        (tree/'DEBIAN').mkdir(parents=True,exist_ok=True)
        (tree/'DEBIAN/control').write_text(f'Package: {name}\nVersion: {version}\nArchitecture: amd64\nMaintainer: Fixture <f@example.invalid>\nDescription: synthetic fixture\n{relationships}')
        output = tree.with_suffix('.deb')
        subprocess.run(['dpkg-deb','--build',str(tree),str(output)],check=True,stdout=subprocess.DEVNULL)
        return output

    def test_gate_checks_both_control_relationships_and_allows_retained_companion(self):
        self.pair('0.6.0','0.5.0')
        main = self.package(MAIN,'0.6.0',f'Breaks: {policy.main_breaks(self.root)}\n')
        islands = self.package(ISLANDS,'0.5.0',f'Depends: {MAIN} (>= 0.5.0)\n')
        policy.verify_pair(self.root,main,islands)
        for requirement in (MAIN, f'{MAIN} (>= 0.7.0)', f'{MAIN} (>= 0.5.0) | alternative'):
            islands = self.package(ISLANDS,'0.5.0',f'Depends: {requirement}\n')
            with self.assertRaises(ValueError): policy.verify_pair(self.root,main,islands)
        islands = self.package(ISLANDS,'0.5.0',f'Depends: {MAIN} (>= 0.5.0)\n')
        main = self.package(MAIN,'0.6.0','')
        with self.assertRaises(ValueError): policy.verify_pair(self.root,main,islands)

    def simulate(self, installed, packages):
        # Each scenario needs its own package cache: APT compares status mtime
        # at second resolution, so same-length rapid rewrites can reuse old state.
        root = Path(tempfile.mkdtemp(prefix='apt-', dir=self.root))
        for relative in ('etc/apt/apt.conf.d','etc/apt/sources.list.d','etc/apt/preferences.d',
                         'var/lib/apt/lists/partial','var/lib/dpkg','var/cache/apt/archives/partial','var/log/apt'):
            (root/relative).mkdir(parents=True,exist_ok=True)
        (root/'var/lib/dpkg/status').write_text(''.join(
            f'Package: {name}\nStatus: hold ok installed\nVersion: {version}\nArchitecture: amd64\nDescription: fixture\n\n'
            for name,version in installed))
        config = root/'apt.conf'
        config.write_text(f'''Dir "{root}";
Dir::Etc "{root}/etc/apt";
Dir::Etc::main "{root}/etc/apt/missing";
Dir::Etc::parts "{root}/etc/apt/apt.conf.d";
Dir::State "{root}/var/lib/apt";
Dir::State::status "{root}/var/lib/dpkg/status";
Dir::Cache "{root}/var/cache/apt";
Dir::Log "{root}/var/log/apt";
APT::Architecture "amd64";
APT::Architectures {{ "amd64"; }};
#clear DPkg::Pre-Install-Pkgs;
#clear DPkg::Post-Invoke;
''')
        return subprocess.run(['apt-get','--simulate','--no-remove','install',*[str(p) for p in packages]],
            env={**os.environ,'APT_CONFIG':str(config),'HOME':str(root),'LC_ALL':'C'},
            capture_output=True,text=True,timeout=15)

    def test_apt_rejects_skew_without_installing_optional_islands(self):
        main = self.package(MAIN,'0.5.0',f'Breaks: {policy.main_breaks(self.root)}\n')
        islands = self.package(ISLANDS,'0.5.0',f'Depends: {policy.companion_depends(self.root)}\n')
        result = self.simulate([(MAIN,'0.4.0')],[main])
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        self.assertNotIn('Inst '+ISLANDS,result.stdout)
        result = self.simulate([(MAIN,'0.4.0'),(ISLANDS,'0.4.0')],[main,islands])
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        result = self.simulate([(MAIN,'0.4.0'),(ISLANDS,'0.4.0')],[main])
        self.assertNotEqual(result.returncode,0,result.stdout)
        result = self.simulate([(MAIN,'0.4.0')],[islands])
        self.assertNotEqual(result.returncode,0,result.stdout)
        self.pair('0.6.0','0.5.0')
        later = self.package(MAIN,'0.6.0',f'Breaks: {policy.main_breaks(self.root)}\n')
        result = self.simulate([(MAIN,'0.5.0'),(ISLANDS,'0.5.0')],[later])
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        self.assertNotIn('Inst '+ISLANDS,result.stdout)
        result = self.simulate([(MAIN,'0.5.0'),(ISLANDS,'0.7.0')],[later])
        self.assertNotEqual(result.returncode,0,result.stdout)


if __name__ == '__main__': unittest.main()

#!/usr/bin/env python3
"""Public sample track on an explicitly supplied private session bus.

No playback, filesystem, artwork or external service access. Used only to keep
the existing Music card visible in the full standalone-widget workload.
"""
import os
import dbus
import dbus.service
from dbus.mainloop.glib import DBusGMainLoop
from gi.repository import GLib

assert os.environ.get('HALCYON_PRIVATE_MPRIS') == '1'
assert os.environ.get('DBUS_SESSION_BUS_ADDRESS')
DBusGMainLoop(set_as_default=True)
bus = dbus.SessionBus()
name = dbus.service.BusName('org.mpris.MediaPlayer2.halcyon_fixture', bus)
ROOT = 'org.mpris.MediaPlayer2'
PLAYER = ROOT + '.Player'
values = {
    ROOT: {'Identity': 'Foundation fixture', 'CanRaise': False},
    PLAYER: {
        'PlaybackStatus': 'Paused', 'Position': dbus.Int64(60_000_000),
        'Rate': dbus.Double(1), 'CanPlay': False, 'CanPause': False,
        'CanSeek': False, 'CanControl': False, 'CanGoNext': False,
        'CanGoPrevious': False,
        'Metadata': dbus.Dictionary({
            'xesam:title': 'Sample track',
            'xesam:artist': dbus.Array(['Sample artist'], signature='s'),
            'mpris:length': dbus.Int64(240_000_000),
            'mpris:trackid': dbus.ObjectPath('/org/mpris/MediaPlayer2/sample'),
        }, signature='sv'),
    },
}


class PausedPlayer(dbus.service.Object):
    @dbus.service.method('org.freedesktop.DBus.Properties', in_signature='s', out_signature='a{sv}')
    def GetAll(self, interface):
        return values.get(interface, {})

    @dbus.service.method('org.freedesktop.DBus.Properties', in_signature='ss', out_signature='v')
    def Get(self, interface, prop):
        return values[interface][prop]


player = PausedPlayer(bus, '/org/mpris/MediaPlayer2')
print('ready', flush=True)
GLib.MainLoop().run()

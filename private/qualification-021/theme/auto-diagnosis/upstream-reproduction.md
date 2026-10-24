# Minimal native clock-step reproduction — untouched comparison still pending

Target: untouched retained Pop!_OS 24.04 / COSMIC 1.11 cohort, settings-daemon
`1.11.0~1792526191~24.04~655b14f`, systemd 255. No Halcyon package or theme.
Use a disposable offline guest. This recipe changes its clock; it is not intended
for the controller/host. Capture the same daemon PID throughout the live steps.

1. Before native user login, disable NTP and set a known supported timezone and
   night time using real timedated operations:

   ```sh
   timedatectl set-ntp false
   timedatectl set-timezone Europe/London
   timedatectl set-time '2026-10-23 23:00:00'
   ```

2. Log into the real native COSMIC session. Open Settings → Desktop → Appearance
   and enable the real automatic theme toggle. Close Settings. Read actual native
   mode config and the portal color-scheme; retain a full screenshot. This checks
   startup scheduling at night, not a live clock transition.

3. Monitor the actual system bus (start before the operations if possible):

   ```sh
   dbus-monitor --system "path='/org/freedesktop/timedate1'" \
     "type='signal',interface='org.freedesktop.DBus',member='NameOwnerChanged',arg0='org.freedesktop.timedate1'"
   ```

4. Without restarting the settings daemon or editing mode keys:

   ```sh
   timedatectl set-time '2026-10-23 12:00:00'
   ```

   Sample the portal/native state for at least forty monotonic seconds. Expected
   day mode is Light. Preserve whether the mode changes, the actual method call,
   any PropertiesChanged signal and the settings-daemon process identity.

5. As a separately labeled location/timezone recomputation test:

   ```sh
   timedatectl set-timezone Europe/Paris
   ```

   Local daytime remains daytime. Record whether native Auto now becomes Light.
   This is not a successful clock-step transition or a product workaround.

6. Repeat a live SetTime to 23:00 in Paris, then separately SetTimezone London.
   Record each native/portal state and actual signals. Neither stimulus is a
   natural sunrise/sunset boundary. Do not restart the daemon to make a live
   transition pass.

From a shell in the actual desktop session, read the portal with:

```sh
busctl --user call org.freedesktop.portal.Desktop \
 /org/freedesktop/portal/desktop org.freedesktop.portal.Settings \
 Read ss org.freedesktop.appearance color-scheme
```

Portal uint32 1 indicates Dark, 2 Light. Compare with
`~/.config/cosmic/com.system76.CosmicTheme.Mode/v1/{auto_switch,is_dark}` when
present; an absent override requires checking the effective portal/default.

Candidate-session evidence in run 39987753102 observed Dark at startup night,
noon SetTime remaining Dark, Paris timezone recomputation to Light, night SetTime
remaining Light, London timezone recomputation to Dark, all with one settings-
daemon PID. Actual SetTime methods had no PropertiesChanged signal. The intended
untouched baseline stopped before login on a wrong empty-DVD assertion, so an
identical upstream-only runtime reproduction is NOT established.

Suspected boundary, with untouched runtime comparison still pending: daemon `src/time.rs` translates
only timedated PropertiesChanged into WallClockChange, whereas the
[systemd 255 API](https://www.freedesktop.org/software/systemd/man/255/org.freedesktop.timedate1.html)
does not promise clock-step notifications from that interface. Actual monitored
absence/presence is required before attributing this retained guest's behavior.
Natural solar switching and suspend/resume are outside this reproduction.

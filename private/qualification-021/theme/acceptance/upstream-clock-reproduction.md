# Confirmed untouched pinned-COSMIC clock-step reproduction

Observed on untouched stock guest in Actions run **39997696992**, artifact
**12521924667**, phase `untouched-clock`. No Halcyon/Islands package, process,
config or inserted candidate media. The same effective sequence was already
observed with frozen candidate in run **39987753102**, artifact **12516872732**.
This is a native limitation of the pinned cohort, not a claim about all versions.
No external issue has been submitted and no product workaround has been made.

Cohort: settings-daemon `1.11.0~1792526191~24.04~655b14f`, source
`655b14fb9572dd7f7231ec6c343c63984e99741a`, daemon ELF
`0f91fafb1eeb97cee09211418059cc5247af06cc130c206147d349f2ebdd7423`;
portal `1.11.0~1792525517~24.04~f448ddf`, systemd
`255.4-1ubuntu8.15pop0~1778766128~24.04~85b5073`, DBus `1.14.10-4ubuntu4.1`.

Use only a disposable guest. Before user login:

```sh
timedatectl set-ntp false
timedatectl set-timezone Europe/London
timedatectl set-time '2026-10-23 23:00:00'
```

Log into actual COSMIC, open native Settings → Desktop → Appearance and enable
Auto through its real switch. Close Settings. Start system DBus monitoring before
the operations, and keep recording the same settings-daemon process:

```sh
dbus-monitor --system "path='/org/freedesktop/timedate1'" \
 "type='signal',interface='org.freedesktop.DBus',member='NameOwnerChanged',arg0='org.freedesktop.timedate1'"
```

Read the actual session portal with:

```sh
busctl --user call org.freedesktop.portal.Desktop \
 /org/freedesktop/portal/desktop org.freedesktop.portal.Settings \
 Read ss org.freedesktop.appearance color-scheme
```

Dark is uint32 1, Light 2. Record native `auto_switch` and `is_dark` overrides
when present. An absent `is_dark` override uses the native default; in this
untouched cohort that default is Dark and is independently confirmed by portal/UI.

1. Initial night: Auto true, effective Dark.
2. `timedatectl set-time '2026-10-23 12:00:00'`: actual clock becomes noon,
   but native appearance remains Dark for forty seconds (observed 41.942 seconds).
3. `timedatectl set-timezone Europe/Paris`: daytime remains daytime; native
   appearance recomputes to Light.
4. `timedatectl set-time '2026-10-23 23:00:00'`: native appearance remains Light
   for forty seconds (observed 41.942 seconds).
5. `timedatectl set-timezone Europe/London`: night remains night; native
   appearance recomputes to Dark.

Do not restart the daemon, write mode keys or emit synthetic mode signals.
Check `date --iso-8601=ns` and timedated state as well as screenshots; the desktop
clock label can itself be stale after a clock jump. Daemon PID **1619** and portal
PID **1983** stay unchanged throughout the observed sequence. Timedated's own
service idles/exits/reactivates separately, which is recorded in raw signals and
service state.

Actual trace has 30 messages, four PropertiesChanged signals (NTP false and
three timezone changes), and no SetTime-related property notification. Native
`src/time.rs` relies on timedated PropertiesChanged for WallClockChange while
solar times use monotonic deadlines in `src/theme.rs`. The
[systemd 255 API](https://www.freedesktop.org/software/systemd/man/255/org.freedesktop.timedate1.html)
does not provide clock-step notification on that interface; local clock-step
observers should use the documented timerfd mechanism. This identifies the
suspected event-source boundary; it is not a Halcyon repair instruction.

Separate clean natural-sunrise tests pass on untouched stock, candidate stock,
and candidate Islands without any post-login clock/timezone stimulus, mode write
or daemon restart. Therefore this clock-jump failure must not be presented as a
failure of ordinary natural solar switching.

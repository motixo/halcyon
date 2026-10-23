# Primary interface reference inspected before diagnostic completion

Systemd 255 official API documentation:
https://www.freedesktop.org/software/systemd/man/255/org.freedesktop.timedate1.html

Retrieved 2026-10-23. The installed guest uses systemd 255; its exact package
identity and actual DBus traffic remain the primary runtime evidence.

The documented TimeUSec and RTCTimeUSec properties do not emit changed signals.
Timezone and LocalRTC changes do notify subscribed clients. The API explicitly
directs local clock-step observers to timerfd with CLOCK_REALTIME and
TFD_TIMER_CANCEL_ON_SET, rather than expecting this service to notify clock steps.

This is the upstream API contract, not proof of a particular observed guest
failure or the precise downstream patch set. The running settings-daemon source
655b14fb9572dd7f7231ec6c343c63984e99741a subscribes to timedated PropertiesChanged
for WallClockChange. The untouched baseline and raw monitored SetTime/SetTimezone
traffic are required to establish the actual boundary in this retained guest.
No product change is warranted by documentation alone.

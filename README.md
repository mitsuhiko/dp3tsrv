# dp3tsrv

This implements a simple backend for
[dp3t](https://github.com/DP-3T/documents/), a proximity tracing system.

It has two endpoints useful:

* `/fetch/timestamp`: fetches all compact contact numbers from that timestamp forward.
* `/submit`: submits a new compact contact number.

Additionally there is an endpoint to check TCNs directly:

* `/check`: checks the last 14 days of CCNs with 1440 TCNs each against the request TCNs.

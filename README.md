# dp3tsrv

This implements a simple backend for
[dp3t](https://github.com/DP-3T/documents/), a proximity tracing system.

It has two endpoints:

* `/fetch/timestamp`: fetches all compact contact numbers from that timestamp forward.
* `/submit`: submits a new compact contact number.

# Reference takeoff of the stock Cessna 172 SP (X-Plane 12.4.3, Data Output)

Recorded 2026-10-30: new flight, "Cessna Skyhawk" (`Cessna_172SP.acf` per `Log.txt`) at UHWW runway 07L, clear
weather, engines started from the Flight menu, parking brake released with `V`, full throttle with `F3`, the
mouse yoke pulled back later. `_Bdisk0,1,3,4,17,18,20` enabled for the run and restored afterwards. The excerpt
`research/reference/c172_takeoff_uhww_20261030.csv` keeps every tenth sample (about 0.2 s) from 2 s before the
roll to 45 s after it: indicated airspeed, VVI, alpha, pitch, roll, heading, height above ground, position.
Column mapping of the original table: 14 Vind kias, 22 VVI, 27 alpha, 32 pitch, 33 roll, 34 heading,
41 height above ground (46 columns per sample, the file wraps each sample over several lines).

## Roll comparison with the port (`fly-test`, full throttle from standstill)
| Speed | Reference, s after the roll starts | Port, s |
|---:|---:|---:|
| 10 kt | 1.7 | 2.0 |
| 20 kt | 4.2 | 4.2 |
| 30 kt | 7.5 | 6.4 |
| 40 kt | 10.9 | 8.7 |
| 50 kt | 14.6 | 11.2 |
| 55 kt | 16.5 | 12.6 |
| 60 kt | 18.6 | 14.2 |
| 70 kt | 23.1 | 18.2 |

The port matches the first 20 kt and then accelerates faster: it reaches 70 kt in 18.2 s against 23.1 s
(about 21 % sooner). This is the assumed thrust law (`PROP_EFFICIENCY`, static cap) not falling off with speed
as the original does, or rolling drag that is too low. The reference liftoff (height above ground over 3 ft)
came 35.5 s after the roll started at 88 kias, but that depends on when the yoke was pulled and says nothing
about the model. The run is one flight with unrepeated pilot inputs, so the numbers are indicative, not a
tolerance. The reference aircraft left the runway sideways in the roll (no rudder input), which the port's
straight run does not show.

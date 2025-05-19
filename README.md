# Trackellite
Satellite tracking TUI with support for multiple ground station and upcoming pass data, available both natively and a TUI and in the browser (coming soon!).

## Installing
To install the TUI app, Binaries are available for some common targets (windows, mac, X86 linux). The program has only been tested on windows using windows terminal and fedora using Gnome terminal, so please open an issue if graphics quality degrades
in other terminals.

If your platform is not available as a pre-built binary, it is almost assuradly still compatable, but will require you to build the project from source.

## Use
To use Trackellite, simply invoke `trackellite` (or `./trackellite`). Satellites can be added to the system by pressing `s`, then inputting either the norad ID of the satellite or the TLE of the satellite. Note that currently,
adding a satellite with norad ID requires an internet connection, and will result in two api calls to celestrak. Adding a satellite by TLE results in one call (to fetch other metadata about the satellite). Satellites (and metadata)
are cached by the program to limit network use, and TLE's are only updated on request.

To add a ground station press `g` and select add station. The Latitude and Longitude coordinates are in decimal degrees, with north and east positive. Altitude is sea level altitude and is in meters.

Trackellite is a 1 satellite multi ground station tracking system, to best serve the needs of satellite operations. As such, passes are computed for each ground station selected in the GS menu simultaniously. Ground stations are also cached
to limit the need for re-entry

Cached data is stored in the system data directory, on linux this is _normally_ `~/.local/share/trackellite/` and consist of a pair of JSON files. Effort is made to minimize disk use, by caching only the direct return from celestrak and no
derived data about the satellite. 

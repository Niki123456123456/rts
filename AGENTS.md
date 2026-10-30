For changes to this game, the user expects a playable version, not only edited
source files. Build and open the local browser game, verify the affected behavior,
and keep SPIEL-STARTEN.cmd working. If publishing is unavailable, explain that the
GitHub Pages site is unchanged and provide the local playable URL.

This landscape uses the bundled local SpacetimeDB server, at
http://127.0.0.1:18766, database stronghold-local. The old Maincloud module does
not have this terrain: do not connect the new landscape to that old module.
Use Build-Spiel.ps1 to rebuild the browser and package it into public/. Pass
-ServerChanged when shared terrain or server logic changes. SPIEL-STARTEN.cmd
starts both the persistent database and the web server. Do not delete local
database data when updating it. Keep credentials and server data private.

Resource rules and foundation heights live in shared/src/terrain.rs. Successful
construction records permanent terrain_clearing rows; keep these on demolition.
The Rust and browser snapshots subscribe to these rows, and terrain::sync_clearings
hides tree crowns and trunks. Regenerate both client bindings and rebuild network.js
when the server schema changes. The focused real-server test is game/web/src/terrain.test.ts;
run it against the isolated stronghold-terrain-test database, not a user's game.
The bundled Node 24 executable is at
C:/Users/nikla/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node.exe;
use it with --import tsx src/terrain.test.ts from game/web on this computer.

Keep geometry is authored by game/create-keep.py and registered in the JSON and
shared building catalogs. Re-run the generator after model edits and copy its
GLBs to public/assets. The keep has one owner, a 5.4 m terrace, physical stair
travel using mode 4, and a public keep_population row. Immigration is server
scheduled with intervals based on satisfaction and free housing. Existing production
jobs are retained. Test the complete flow with game/web/src/keep.test.ts against
stronghold-terrain-test. The sharing gateway must refresh its compressed wasm
after browser builds, or the public link can serve an old client.

Each participation gets one automatically placed keep through ensure_player_keep
at join/create, with connected backfilling active players. player_keep_spawn is
a private one-time grant marker; delete it on leave, not on building demolition.
The client focuses its own keep once its game subscription delivers the building.
Do not revert to requiring manual keep construction at game start.

Satisfaction is calculated in shared/src/needs.rs and published in settlement_needs.
Migration now follows satisfaction, food is consumed from actual granaries, and
only free peasants may emigrate. Buildings can be constructed without workers:
register_logistics_building creates the producer; auto_assign_peasants staffs open
buildings whenever peasants arrive or a building is placed/opened/closed. The
public building_operation table records closed buildings (missing row = open).
Legacy assign_peasant / unassign_peasant reducers now mean open / close. Never
restart a closed workplace automatically; never create duplicate workers. Production requires an assigned worker at the
workplace. Count employed population from worker records, never building demand.
Amenity models have the services UI category, retaining their original asset paths.
The focused real-server test is game/web/src/needs.test.ts. Keep both native and
browser snapshots/actions in sync when changing workforce or needs records.


## Shared GitHub workflow

The user authorizes uploading completed changes directly to
Niki123456123456/rts and wants collaboration kept in sync. Before editing, fetch
origin and inspect incoming commits; integrate them without overwriting local
work. After relevant checks, commit and push the changes to main. Fetch again
before pushing; if main advanced, integrate and recheck affected behavior. Never
force-push shared history. Report the uploaded commit and any pending conflicts.
This workflow applies whenever working on this project. Do not run scheduled
synchronization: fetch and integrate collaborator changes when preparing each
upload. Do not auto-commit unfinished changes. Never upload credentials, database
data, runtime executables, build caches, or generated browser binaries.

Pages deployment requires repository variables SPACETIMEDB_URI and
SPACETIMEDB_DATABASE pointing to a server updated with this module. Until both
are set, CI builds and tests the source but does not replace the public site.
The previous Maincloud module is incompatible with the new client schema.


Satisfaction has population-based stages in shared/src/needs.rs::NeedsStage.
The first four settlers have no food-shortage penalty; water activates at 8,
variety and employment at 16, health at 24, leisure at 32, goods at 40.
The clickable happiness window displays the same thresholds and base score.
Keep terrace occupants must take selection priority over the full-height building
proxy, and that proxy must not block roof movement commands. Keep travel mode 4
also allows a new ground order while crossing its external upper stair landing.
keep.test.ts covers lord, armed peasant and archer physical ascent/descent.

Camp positions and arrival paths live in shared/src/camp.rs. Both waiting visuals
and automatically assigned workers must use free camp positions; no straight-line
arrival animation through buildings and no fallback spawning at the workplace.
The client caches/replans arrival routes when building obstacles change.
Nearby stone/iron deposits, forest, desert and small mountains complement the
original landscape. Keep those resource plots buildable and the starting clearing
free. needs.test.ts now verifies automatic staffing, close/open persistence and
actual worker positions while routing around a building, plus nearby resource jobs.


The keep wire ID now represents the automatically granted PALACE (Palast).
Do not expose manual palace construction. TowerGreat is the separate round
Bergfried; TowerLookout uses a 1.4 vertical model scale and the matching roof
height, making the narrow lookout the tallest tower. Keep old wire IDs stable.
All buildings use shared Cost; recruitment consumes both materials and a free
peasant. StartingSupplies is a private one-time migration/grant marker, cleared
on leave. Warehouse initialization transfers remaining starting supplies; never
add the starting balance again when a warehouse is built.

Living resources are in shared/server/game ecology.rs. ResourceNode.site is the
stable deterministic tree/deposit/fish key. Harvesting must consume these rows,
not create unlimited producer buffer. Fish regenerates slowly; terrestrial
resources stay depleted. OxCart links one OxTether to one raw-material producer.
Stone/iron loads require a cart; cart access points must satisfy work_range as
well as inflated collision clearance or they can get stuck outside delivery range.
FishStock is additive so existing GranaryStock rows remain upgrade-compatible.
Wildlife includes herds, corpses, hunting, territorial defence and reachable-camp raids.

Fortifications use shared/server/game fortifications.rs. GateState drives actual
server/client gate collision and animation. Opening a drawbridge cuts a narrow
corridor through conservative river/moat obstacles and uses the real deck height.
Moat centres align to the terrain grid; client terrain indices cut real holes.
WallStair stores the ramp upper endpoint, with a horizontal landing to wall_id.
Both away and parallel stairs use continuous travel mode 4, including reversing
an order while on the upper landing. Reserve the entire ramp against construction.
Walkable roof slot bounds must fit the actual model floor. Merlons are 3D arrow
blocks. Building construction reserves both logical plots and visible bounds;
wall joins intentionally share mortar edges. Keep snapshots/actions in both
native Rust and TypeScript synchronized. Regenerate gate leaves with
prepare-gates.py; procedural walls/bridge come from create-fortifications.py.
Run ecology.test.ts, fortifications.test.ts and the existing needs/keep/terrain
real-server tests against stronghold-terrain-test, never the player's database.

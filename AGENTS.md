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
register_logistics_building creates the producer but must never auto-assign a
worker on placement or connection. assign_peasant / unassign_peasant explicitly
manage one worker per producer. Production requires an assigned worker at the
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
This workflow applies whenever working on this project; it is not a continuously
running background synchronization service. Never upload credentials, database
data, runtime executables, build caches, or generated browser binaries.

Pages deployment requires repository variables SPACETIMEDB_URI and
SPACETIMEDB_DATABASE pointing to a server updated with this module. Until both
are set, CI builds and tests the source but does not replace the public site.
The previous Maincloud module is incompatible with the new client schema.

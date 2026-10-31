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


New buildings must use Roman/antique architecture (stone, stucco, terracotta tiles),
not medieval forms. The small Roman fishery is generated by create-fishery.py;
its entire plot must be water near a fish school. Workers operate from a reachable
river bank. PredatorRevision migrates old oversized packs and remote homes;
PredatorPeace is retained only for save compatibility. Raids now roll a rare chance
each second without a minimum age; keep predators sparse and remote at spawn.
Lord health uses UnitKind::max_health; profession defense is shared ecology::defence.
Manual save_game/load_game snapshots are private, owned by Identity and create a
new world on load. Never expose SavedWorld or authentication tokens. Browser
authentication persists only in localStorage, namespaced by server/database.
Keep native/TypeScript SaveCatalog snapshots and actions synchronized.

Browser cache.ts cancels inverse operations on intermediate serialized rows before
SDK 1.1.0 applies a transaction. Keep this compatibility fix and its cache.test.ts
regression test: repeated updates during loading/damage otherwise leave ghost rows.

Palace now has mirrored physical staircases: door 0 east, door 1 west, with
hatch_for_door selecting the matching landing. Keep model openings, banner
clearance and traversal routes in agreement. Flags must stay above the main roof.
Production/supply placement repeats until right click or Escape. Owned buildings
can be demolished by the visible button or Delete outside text inputs.
New test games start with 120 of each material and 120 sesterces. Existing stock is preserved. Tree wood
is 24, harvesting intervals 6 seconds wood/stone and 7 iron. Deposit capacities
come from actual terrain field area and density. resource_sizing_v2 migrates
remaining deposits once without resurrecting exhausted trees or fields.
Cart matching is automatic within 80 m, same owner, open staffed raw-material
workplaces; complete carried loads before reassignment. Cart plots allow mineral
fields. Never expose manual assignment as a required player step.

Civic policy/economy lives in shared/server/game civic.rs. The palace window has
8 rations and 8 tax levels; effects are authoritative. Treasury uses integer
sesterces, income per resident every 30 seconds. MarketPressure is game-wide;
quotes include marginal demand and spread. market_exchange is an atomic sale
and purchase with change, not free resource creation. Requests only transfer on
recipient acceptance, enforce same game/ownership, and cannot be accepted twice.
CivicSave is a private sidecar to SaveSlot: keep SavedWorld wire schema stable
for older saves; remap civic owners on load/claim. No authentication data in Git.
Alarm pauses worker production/dispatch, routes workers physically into palace,
then retries closing unoccupied gates. Wait for roof travel before resuming jobs.
Browser horn synthesis is in network.ts. Waiting camp visuals use both stairs.

Fortification attachments/refunds and exposed parapet edges use shared
fortifications::Structure. Only own anchors replace own walls; occupied wall
roofs reject replacement. Wall drag skips anchor-covered cells without charge.
Same-owner touching plots seal physical gaps with shared joins, including arrow
collision; equal-height roof surfaces have direct paths. Crenellated wall meshes
have no baked crenels: game sync and server shot_blocks use shared parapets.
Small gates also have physical crenels, leaving their roof hatch clear.
WoodenWall is appended to the catalog to preserve all existing wire IDs.
Roman market comes from create-market.py. Test new logic using civic.test.ts,
attachments.test.ts and balance.test.ts against isolated databases; test-world
allows RTS_TEST_DATABASE (default stronghold-terrain-test). Never use player data.

Direct building attachment uses shared placement::snap on client and server.
Construction collision follows visible bounds rather than oversized grid plots;
wall sections retain mortar joins. Do not reintroduce mandatory spacer cells.
Ground units still cannot cross touching building bodies. New light_swordsman
and light_cavalry kinds are appended string kinds; cavalry movement uses shared
UnitKind::speed and melee uses melee_dps. Packaged sword rig plus animated horse
variants are reproducible with game/create-light-units.py. All five rigs must
load, animate and retain hand-attached weapons. Wall menu icons are material and
height silhouettes in menu_ui.rs, not the generic stone atlas tile. Building
hover hints show the building name, material icons/counts and controls. Treasury HUD displays a coin and
signed net per 30-second round; current recurring income is tax, with no base
expenses yet. Shared civic::net_per_round is the displayed balance calculation.

Build selection opens build_info.rs on the right, above the current minimap.
Use resources::cost_icons for material prices instead of spelling out each material.
Shared fortifications::joins fills roof-to-roof tower seams and raster remainders
between wall ends and any nearby own building. Keep its continuous solid core,
server/client collision, and occupied-extension construction rejection in sync.
Tower battlements are baked into original GLBs: tower_mesh.rs clips only the
facing region above the floor for same-level tower neighbours. Never mutate a
shared source mesh or remove all parapets. Instance-local variants preserve
float vertex attributes, restore originals on neighbour removal, and are cached
until structures change. Visible battlements may extend higher than collision
GLB bounds: do not cap trimming at BuildingKind::height alone. Test seams.test.ts
on an isolated database, plus tower_mesh and fortifications attachment tests.

Roof hatches must remain inside the walkable area with unit-radius clearance.
BuildingRoof::hatch_for_door clamps the approach point; preserve both palace
stairs and verify entry/exit routes for every roof profile, not only square towers.

Roman industry is shared/server/game industry.rs. Wheat (8) and flour (9) are
intermediate cargo, stored in additive IndustryStock at real stockpiles. WheatFarm
produces grain, Windmill (Mola) consumes grain, Bakery (Pistrinum) consumes flour
and delivers Bread (4) to a granary. Stable and SiegeWorkshop use the producer
workforce but must never create or dispatch placeholder wood. StableHorse ages
on migration ticks; staffed stables breed every 45 work seconds, foals mature at
90 seconds. Cavalry consumes a real adult horse. Fabrica queue_siege charges
materials and two free peasants, builds Ballista/Onager over 30/50 work seconds.
Siege projectiles retain their kind in ProjectileStyle until impact. Harbor
(Portus) is a river-bank trade building, not a ship simulation. New Roman GLBs
come from create-industry.py; horse rigs from create-light-units.py. IndustrySave
is a private sidecar: do not change SavedWorld or expose authentication data.

Crenellated walls default to 10 m (same as StoneWallHigh). Three appended hidden
variants match directly adjacent lower plain walls; reconcile_fort_heights updates
occupants/travel/stairs and client models. FortHeightRevision migrates old floors
once. Shared doorways/roof_transfer open higher tower walls at the lower floor;
mode 5 travels internally through that tower. Initial old-roof route segments use
mode 4, not destination-roof collision/height. masonry.rs continues the actual
neighbouring tower material and world UV scale onto solid joints and door frames;
keep original meshes/materials unchanged. Run industry.test.ts,
siege-industry.test.ts and tower-doors.test.ts against stronghold-industry-test,
plus shared tests and native material/horse/catalog tests.

Roman expansion uses shared/server/game advancement.rs, server/workshops.rs,
server/infrastructure.rs and server/siege.rs. ArsenalStock, ResearchState,
ResearchOrder, WorkshopStock, WorkshopRecipe and SiegeAccess are additive tables.
AdvancedSave and SiegeClimberSave are private save sidecars; never change the
original SavedWorld wire shape. Remap all building/unit IDs when restoring.
Wine and oil are real workshop cargo, consumed optionally from population 16;
tools wear out during work. Weapon stock discounts recruitment materials once.
MercenaryCamp recruits for gold without consuming the peasant pool. Hospital
requires a worker and heals nearby own units up to their actual maximum health.

Transport uses build_wall_kind with dirt_road, stone_road or roman_bridge.
Roads are nonblocking, have paid/idempotent segments, and change real server
movement speed. Bridges require dry endpoints and cut river collision only at
their footprints. Aqueduct piers stay solid while arches remain walkable.
Ladders/towers establish SiegeAccess at enemy fortifications; assault_roof and
SiegeClimber provide continuous entry and return travel. Siege crews and research
requirements are defined in shared units/advancement, not hard-coded as two crew.

AttackAlert tracks actual damage and expires after combat. Browser horn/warning
synthesis is in game/web/src/sounds.ts, with input-triggered audio unlock. Minimap
alerts only reveal owned targets. The build menu uses eight two-column groups
and procedural silhouettes for new Roman structures. Run roman-industry,
roman-goods, roman-hospital-alerts, roman-transport, roman-siege,
roman-heavy-siege and roman-horses tests against stronghold-roman-test; run
sounds.test.ts, cache.test.ts, shared tests and native asset/collision tests.

# Stronghold multiplayer server

The module uses **SpacetimeDB CLI/server 1.1.2**, matching the installed CLI.
Use that version for publishing and regenerating client bindings.
`game/src/module_bindings/` and `game/web/src/module_bindings/` are generated and
ignored by Git. CI installs the pinned code generator and regenerates both before
checks and compilation. The browser build script also generates both automatically.
The matching published SDK versions are Rust 1.1.1 and TypeScript 1.1.0.
Commit both Cargo lockfiles and `game/web/package-lock.json` when changing dependencies;
the SpacetimeDB protocol and macro dependencies must stay on 1.1.2.

## Hosted server

The game defaults to the deployed Maincloud database:

- Endpoint: `https://maincloud.spacetimedb.com` (browser connections use WSS).
- Database: `stronghold-niki123456123456`.
- Identity: `c200894545848c3e5c08b3cb805ab368e24ab6e292974790a6ba1656cb95371b`.
- Example game: `GameA`; each joining player receives 20 units.

Maincloud has a [Free plan](https://spacetimedb.com/pricing) with 2,500 TeV/month
of usage credit. This deployment did not select a paid plan or enable billing.
Actual usage still depends on players, movement, subscriptions and storage; the
free credit is not unlimited server time. The movement timer stops when idle.

Cloud credentials are kept in the ignored `server/.spacetime/maincloud-cli.toml`,
separate from the default CLI login used by the local server. On a new machine:

```sh
mkdir -p server/.spacetime
spacetime --config-path "$PWD/server/.spacetime/maincloud-cli.toml" login
```

To update the hosted module without deleting existing games:

```sh
bash server/publish-maincloud.sh
```

Public deployment settings are in `server/maincloud.env`. The game also accepts
`SPACETIMEDB_URI` and `SPACETIMEDB_DATABASE` overrides at build time, or at runtime
for the native game; the lobby fields allow changing the destination manually.

## Local server

From the repository root, start a persistent local server:

```sh
bash server/start.sh
```

In another terminal publish the module, then start the game:

```sh
spacetime login --server-issued-login http://127.0.0.1:3000
bash server/publish.sh
bash server/generate-bindings.sh
cd game
SPACETIMEDB_URI=http://127.0.0.1:3000 SPACETIMEDB_DATABASE=stronghold cargo run --locked
```

The command above overrides the game's Maincloud defaults to use the local server.
Players can change these fields and click Connect/Reconnect. The server must already
be running and the module published. Run two clients or browser tabs, create a game
in one, and join that game in the other. Each player receives 20 units in a distinct color and can command only their own army. Games persist even when everyone leaves.

For another server/database, set `SPACETIMEDB_URI` and `SPACETIMEDB_DATABASE` when
publishing and running the native game. Set them when building the web game to
change the browser's defaults. For LAN play, clients use the server computer's IP
instead of `127.0.0.1`. An HTTPS website needs an HTTPS/WSS SpacetimeDB endpoint;
GitHub Pages only hosts the client, so run the database separately.

## State and authority

- `game`: unique title and ID.
- `army_unit`: game ID, owner connection, color, unit number, `health: f32` initially
  `1.0`, current `x/y/z`, and target `x/z`.
- `participant`: one active game and color per connection; its army is removed on
  leave, disconnect or game switch. Rejoining the same game does not duplicate units.
- Legacy `unit` and `player` tables remain only to allow a non-destructive upgrade
  of the existing hosted database; the new client does not use them.
- `building`: game ID, owner connection, player color, kind (`granary`, `barracks` or `stockpile`), `x/y/z`,
  and `health: f32` initially `1.0`.
- Private `unit_order`: unit ID, game ID, waypoint list and optional attack target.
- `building_rotation`: building ID, game ID and `quarter_turns` (0–3). Buildings
  without a rotation row use 0; existing building records remain unchanged.
- Private `movement_tick`: 50 ms server schedule, active only while units move or attack.
- `tower_occupant`: per-unit stair-tower slot and doorway reservation, plus an
  optional destination slot during roof repositioning or transfer to another tower.
- Private `tower_travel`: explicit 3D stair/hatch waypoints and roof/ground phases.

`create_game` creates the lobby entry and joins the creator. `join_game` allocates
a player color and spawns a 5×4 army of 20 units.
`set_targets` accepts only IDs and a destination. It checks membership, unit ownership, map bounds,
finite coordinates, and unit count, then calculates formation offsets from server
positions. Clients cannot set current positions, speed, or tick duration.
`set_formation_targets` accepts owned unit IDs and the drag endpoints. Shared geometry
calculates parallel rows centered on the vector midpoint, with row width equal to
the drag length. The server assigns nearby units to exact preview slots and rejects
blocked, occupied, out-of-bounds or unreachable formations atomically.
The protected scheduler advances units at 3 world units/second and computes terrain
height. Clients subscribe to the lobby and only their active game's units and buildings, and render
the received positions without simulating movement. Movement continues without clients.
When all units finish moving and attacking, the schedule stops until the next order,
so idle databases do not continuously consume cloud function calls.
`build(x, z, building)` validates the catalog building name and membership, then
snaps finite coordinates to the global 2-m construction grid. Odd cell counts use
odd-metre centres and even cell counts use even-metre centres. Construction overlap
uses the explicit logical `grid` size from `game/building-models.json`; touching
plot edges are allowed, including two 4×4-m stair towers occupying 8×4 m together.
Every occupied cell is sampled for level ground. Units in either the plot or the
physical footprint block construction so model overhangs cannot trap units. There
is a 128-building limit per game. `remove_building` only accepts the owner's buildings.
Buildings are removed with their owner's army on leave, disconnect or game switch.
`build_rotated(x, z, building, quarter_turns)` applies the same checks with a 0–3
quarter-turn orientation. Odd quarter turns swap plot dimensions before snapping,
terrain checks and overlap checks. Navigation and attacks use the rotated physical
footprint. Removing or destroying a building also removes its rotation record.

The supplied collision models are enclosed by axis-aligned XZ footprints.
`shared/` stores each building type’s bounds and model center at scale 0.01, used
by selection, navigation and attack positions. Logical construction footprints
are independent of those mesh bounds. Irregular collision meshes and models without supplied collision meshes use
conservative rectangular footprints. `shared/src/building_catalog.rs` is generated
from `game/building-models.json` by `game/import-buildings.py`.
The client sends one `build` action for all building types, including `rotation`
in quarter turns, and forwards it to `build_rotated`. The original `build` reducer
and three legacy building-specific reducers retain orientation 0 for older clients.
The map is 800×800 world units. Movement stays inside ±399; complete grid plots
may extend to the world edges at ±400. Existing buildings keep their stored
positions when updating the module.
A visibility graph with Dijkstra computes shortest polylines around footprints
expanded by the unit radius plus clearance. Unreachable/blocked orders are rejected.
Construction and demolition replan active orders; units stop if a destination becomes
unreachable. `attack_building` accepts only owned units and an enemy building in the
same game, assigns distinct positions around all four reachable walls, and applies
0.05 health
damage per unit per second only after reaching the assigned position within 0.8
world units of the footprint. Attack routes pass outside the occupied perimeter
before approaching their wall. Additional attackers wait in spaced outer rings
when the melee perimeter is full. At zero health the building is deleted and
routes are updated. Movement orders replace attack orders.

Units keep at least 0.55 world units between their centres, including units owned
by different players. Swept-circle checks prevent crossing through another unit,
local visibility routing finds ways around nearby bodies, and virtual route corners
can be passed without every unit converging on the same point. Spawning checks
occupied positions. Movement destinations reserve free space and adjust destinations
that would overlap another unit or an existing destination.
Clients cannot set damage, health or movement paths. The simulation timer stops when there are no active orders.

`move_to_roof(unit_ids, building_id, x, z)` accepts owned units and a point on an
owned stair tower, lookout tower or square tower's roof. The server reserves
slots at a fixed 0.7 m pitch, with 0.27 m clearance to roof edges: 5×5 (25) for
stair/lookout towers and 8×8 (64) for square towers;
excess units keep their orders. Incoming units and transfers reserve slots before
moving. The original 5×5 slot IDs retain their positions on smaller roofs. On connection
and simulation ticks, saved roof occupants and in-flight targets are refitted to
the current layout. Door and hatch
coordinates in `shared/src/building_roof.rs` follow building rotation. Units walk to
an accessible door and teleport instantly to the hatch, then walk to their slots.
Every traveler advances independently; there is no tower-wide queue or transfer
limit. Roof arrivals may pass through one another while moving from the shared
hatch; distinct reserved slots prevent stacking at rest. Profiles in
`BuildingKind::roof_profile()` define each model's roof height, bounds,
hatch and door list; grid dimensions are derived from the bounds using a shared
unit spacing. The roof uses the original mesh. Ground/attack
orders walk to the hatch and teleport back to free ground
at the door before executing their normal routes. Multiple arrivals/departures can
teleport in the same server tick. Old in-flight stair routes are upgraded to teleports.
A blocked door causes units to choose another door when available; a blocked
single entrance rejects the order. Removing a tower evacuates its occupants onto
free ground. Leaving a game
removes reservations and travel orders. The scheduler sleeps when both ground
orders and tower travel orders are empty.

Run `SPACETIMEDB_DATABASE=<test database> npm --prefix game/web run test:roof` against
a disposable local server to verify capacity, door/hatch teleportation, concurrent transfers, 5×5 slot geometry, departure, rotated towers, multiplayer replication, transfers and cleanup.

Run `npm --prefix game/web run test:roof-types` with the same database environment
to verify model heights, all three tower types in four orientations, inter-tower
transfers, blocked entrances and both additional models through the browser bridge.

## Development and checks

On a fresh checkout, and after changing tables/reducers:

```sh
bash server/generate-bindings.sh
bash server/publish.sh
```

Unit and compile checks:

```sh
bash server/generate-bindings.sh
cd server
cargo fmt --check
cargo test --lib --locked
cargo test --manifest-path ../shared/Cargo.toml --locked
cargo build --locked --release --target wasm32-unknown-unknown
cd ../game
cargo fmt --check
cargo test --locked
cargo check --locked --target wasm32-unknown-unknown
cd web
npm ci
npm run check
npm run build
```

The `--lib` test isolates the pure movement/terrain functions from the WASM host API.
For a live integration test, start a **disposable** local server and publish a test
database (the test creates games):

```sh
spacetime start --in-memory --data-dir /tmp/stronghold-test --listen-addr 127.0.0.1:3300
# In another terminal, from the repository root:
SPACETIMEDB_URI=http://127.0.0.1:3300 SPACETIMEDB_DATABASE=stronghold-test bash server/publish.sh --anonymous
bash server/generate-bindings.sh
cd game/web
npm ci
npm run test:multiplayer
npm run test:buildings
# Optional native SDK round-trip against the same disposable database:
cd ..
cargo test --locked native_client_receives_server_movement -- --ignored
```

The integration test uses two real connections to verify the shared lobby, movement,
20-unit armies, colors, initial health, ownership and game isolation, input validation,
scheduler protection, leaving, and disconnect cleanup. The building test additionally
checks placement, overlap, ownership, route safety, live replanning, damage, destruction,
game isolation and browser building actions.

### Stone walls

`build_wall(start_x, start_z, end_x, end_z)` constructs an inclusive four-connected
2 m raster line in one transaction. Existing owned wall cells can be reused;
terrain, plot overlap, ground units and map boundaries are validated for every
new cell. Wall segments have their own 4096-per-game limit; buildings retain 128.
`build` / `build_rotated` also accept `stone_wall` for a single cell.

The shared wall graph uses full side contact on equal ground elevations. Only
owned walls and the three supported owned tower types participate. `move_to_roof`
accepts a wall ID and fills nearest available slots across its connected component
(nine per segment). Ground entrants use reachable tower doors and hatch teleports;
wall travel follows segment centres at the stair-tower roof height. Higher towers
transfer internally between their hatch and wall level. `wall_entry` is additive
private metadata identifying the approach tower; existing table schemas and saved
games remain compatible. Travel modes 4 and 5 represent wall walking and internal
height transfers. Removing a connection cancels crossing routes; isolated idle
occupants retain their places but cannot descend without a new tower connection.

Run `SPACETIMEDB_DATABASE=<local-database> npm --prefix game/web run test:walls`
against a disposable local server to check raster lines, atomic rejection, capacity,
all three tower access types, surface transfers, severed connections, ground
navigation, ownership, demolition and replication.

The supply catalog contains 37 civilian buildings, including food chains, raw
materials, civilian workshops, storage, trade, water and sanitation. All use the
existing `build` / `build_rotated` reducers with their catalog IDs and logical
2 m plot dimensions; no new reducers or table migrations are needed. Importing
these models alone does not enable their production chains; quarry, iron mine and saw pit now produce raw materials. The real-server
building test checks placement, grid alignment, collision, navigation targets,
combat approach and removal for every newly imported supply kind.

### Resource balances

The additive public `player_resources` table is keyed by participant connection and
indexed by game. Each participation starts with 20 wood, stone and iron. Native and
browser snapshots expose only the local participant's balance. Rejoining the same
active game never replenishes stock; leaving deletes the balance alongside membership.
For hosted upgrades, missing balances of existing participants are initialized once,
without modifying any existing table schemas or resetting existing balances.

`BuildingKind::stone_cost()` is the shared source of prices: 1 stone per `stone_wall`
cell, 5 for each of the seven tower types, zero for other models. All construction
reducers debit the participant's balance after placement validation. SpacetimeDB
transactions roll back both buildings and debits if any segment fails or the total
cost cannot be met. Reused own wall cells are not charged. Demolition has no refund.
Clients cannot write balances. Construction also debits physical warehouse stock; wood and iron construction costs are not enabled.

`npm --prefix game/web run test:resources` against a disposable local database checks
starting balances, ownership, atomic rollback, zero balances, wall reuse, repeat joins,
cleanup and browser snapshot replication.

### Storage and automatic transport

Additive `warehouse_stock`, `producer`, `worker` and private `logistics_player` tables
keep existing schemas intact. A participation gets one starting stockpile with its
20/20/20 stock. Existing balances are preserved and moved into storage once on upgrade.
The shared `logistics` module defines the 24-cell grid, capacities, exact model layout,
production interval (one item/3 seconds), buffer (20), and carrying capacity (5).

Each quarry, iron mine and saw pit creates one server-controlled peasant in
`army_unit`, distinguished by a `worker` row. All player order reducers reject these
units. Production runs in a scheduled one-second reducer; transport uses existing
50 ms movement, pathfinding, obstacle replanning and unit avoidance. Material enters
the spendable balance only on delivery. Full, missing or unreachable storage causes
workers to wait/retry. Storage demolition removes its stock from the balance;
producer demolition removes its worker. Leaving cleans up physical stock,
producers, workers and initialization marker.

`npm --prefix game/web run test:logistics` against a disposable local database checks
physical starting stock, every producer, worker movement and order rejection,
deliveries around an obstacle, observer replication, physical costs and rollback,
demolition, fresh participation and browser snapshots.

### Typed combat units

The additive public `unit_profile` table stores kind (`lord` or `armed_peasant`),
activity (0 idle / 1 walking / 2 fighting) and normalized facing direction. New
participants receive one lord. Existing armies are preserved on upgrades and get
profiles on connection: number 1 becomes a lord, others armed peasants.
`recruit_unit(building_id, kind)` recruits armed peasants at an own, same-game
barracks. It validates the type, ownership, army limit (200 excluding workers),
front spawn positions after rotation, obstacle clearance and unit reservations
atomically. Recruitment is currently immediate and free. Profiles are removed
with participation. Activity is computed from actual movement and attack range,
including switching to idle when the last movement order completes.

`npm --prefix game/web run test:combat` checks starting types, recruitment,
ownership, free front positions, movement/idle/combat transitions, damage,
observer replication, browser actions and cleanup against a disposable server.

### Unit melee and automatic defense

The additive private `unit_engagement` table tracks the selected enemy group,
current target, automatic/manual mode and return position. `attack_units` validates
command ownership, same-game hostile combatants, accessible surfaces and reachable
approach slots. All other explicit unit commands cancel the engagement transactionally.
Group connectivity and distances live in `shared/src/combat.rs` and are shared with
the client's hover preview. Groups use 3 m spatial links and never cross ownership.

`combat_tick` runs every 250 ms while opposing combatants or returning units exist.
Idle units acquire reachable enemies within 8 m; automatic pursuit has a 16 m leash.
Approach slots use standard ground navigation, collision avoidance and reservations;
rear rows wait for melee space. Contact needs range <=1.05 m, height difference
<=0.6 m and unobstructed ground line of sight (or a continuously supported segment on connected roofs). Damage is resolved
simultaneously each scan: lord 0.22/s, armed peasant 0.14/s. A unit at zero health
releases its movement, profile, engagement and roof reservations. Workers do not
participate. Returning units find a free location near their original position.
The scan scheduler stops when no opposing armies or return orders remain.

`npm --prefix game/web run test:melee` checks manual group attacks, reciprocal
combat/animation states, damage, deaths, ownership, automatic acquisition and return,
moving targets, pursuit limits, explicit-command override, wall navigation and
browser attack actions against a disposable local database.

Wall/tower garrisons are reachable for unit combat through enemy tower doors and
hatches. `combat_surfaces.rs` reserves free slots on the physical roof component,
plans surface movement and remembers roof homes in the additive private
`combat_surface_home` table. Movement stops at melee contact, respects peers and
does not bypass missing floor. Roof normalization leaves active combat positions
alone. Client hover validates network access/capacity and shows an error for
isolated or inaccessible walls. Normal roof-movement ownership checks stay intact.
`test:wall-combat` covers ascent, counterattack, returning to the original wall
slot, full garrisons and disconnected fortifications.

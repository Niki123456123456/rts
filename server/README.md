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
- Private `movement_tick`: 50 ms server schedule, active only while units move or attack.

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
`build_granary`, `build_barracks` and `build_stockpile` validate membership, finite coordinates, full footprint within the
map, level terrain, no overlapping buildings and no units in the footprint. There
is a 128-building limit per game. `remove_building` only accepts the owner's buildings.
Buildings are removed with their owner's army on leave, disconnect or game switch.

The supplied collision models are enclosed by axis-aligned XZ footprints.
`shared/` stores each building type’s bounds and model center at scale 0.01, used
by placement previews, overlap checks, navigation and attack positions. The barracks
footprint encloses its irregular collision mesh conservatively.
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

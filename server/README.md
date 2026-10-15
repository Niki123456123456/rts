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
- Example game: `GameA`, with 100 shared units.

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
in one, and join that game in the other. Every player in a game can command its
shared 100 units. Games persist even when everyone leaves.

For another server/database, set `SPACETIMEDB_URI` and `SPACETIMEDB_DATABASE` when
publishing and running the native game. Set them when building the web game to
change the browser's defaults. For LAN play, clients use the server computer's IP
instead of `127.0.0.1`. An HTTPS website needs an HTTPS/WSS SpacetimeDB endpoint;
GitHub Pages only hosts the client, so run the database separately.

## State and authority

- `game`: unique title and ID.
- `unit`: game ID, unit number, current `x/y/z`, and target `x/z`.
- `player`: one active game per client connection; removed on leave/disconnect.
- Private `movement_tick`: 50 ms server schedule, active only while units move.

`create_game` seeds units and joins the creator. `join_game` changes membership.
`set_targets` accepts only IDs and a destination. It checks membership, map bounds,
finite coordinates, and unit count, then calculates formation offsets from server
positions. Clients cannot set current positions, speed, or tick duration.
The protected scheduler advances units at 3 world units/second and computes terrain
height. Clients subscribe to the lobby and only their active game's units, and render
the received positions without simulating movement. Movement continues without clients.
When all units reach their targets, the schedule stops until the next movement order,
so idle databases do not continuously consume cloud function calls.
There is no combat, unit ownership, or obstacle pathfinding yet.

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
# Optional native SDK round-trip against the same disposable database:
cd ..
cargo test --locked native_client_receives_server_movement -- --ignored
```

The integration test uses two real connections to verify the shared lobby, movement,
game isolation, input validation, scheduler protection, leaving, and disconnect cleanup.

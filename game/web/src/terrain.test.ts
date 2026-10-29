// Run against a separate local test database; never changes an existing game.
import assert from 'node:assert/strict';
import { DbConnection, type ReducerEventContext } from './module_bindings';
const uri = process.env.SPACETIMEDB_URI ?? 'http://127.0.0.1:18766';
const database = process.env.SPACETIMEDB_DATABASE ?? 'stronghold-terrain-test';
const clients: DbConnection[] = [];
const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
async function until(test: () => boolean, label: string, timeout = 10000) {
  const deadline = Date.now() + timeout;
  while (!test()) { assert.ok(Date.now() < deadline, label); await delay(20); }
}
async function connect() {
  return new Promise<DbConnection>((resolve, reject) => {
    DbConnection.builder().withUri(uri).withModuleName(database)
      .onConnect(db => {
        clients.push(db);
        db.subscriptionBuilder().onApplied(() => resolve(db)).onError(ctx => reject(ctx.event))
          .subscribe(['SELECT * FROM game', 'SELECT * FROM participant', 'SELECT * FROM building', 'SELECT * FROM terrain_clearing', 'SELECT * FROM keep_population']);
      }).onConnectError((_ctx, error) => reject(error)).build();
    setTimeout(() => reject(new Error('Connection timeout')), 10000).unref();
  });
}
function status(db: DbConnection, register: (cb: (ctx: ReducerEventContext) => void) => void, call: () => void) {
  return new Promise<ReducerEventContext['event']['status']>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Reducer timeout')), 10000);
    let done = false;
    register(ctx => {
      if (done || ctx.event.callerConnectionId?.toHexString() !== db.connectionId.toHexString()) return;
      done = true; clearTimeout(timer); resolve(ctx.event.status);
    });
    call();
  });
}
try {
  const a = await connect(), b = await connect();
  assert.equal((await status(a, cb => a.reducers.onCreateGame(cb), () => a.reducers.createGame(`Terrain verification ${Date.now()}`))).tag, 'Committed');
  const membership = Array.from(a.db.participant.iter()).find(p => p.connectionId.toHexString() === a.connectionId.toHexString())!;
  const game = membership.gameId;
  assert.equal((await status(b, cb => b.reducers.onJoinGame(cb), () => b.reducers.joinGame(game))).tag, 'Committed');
  const buildings = (db = a) => Array.from(db.db.building.iter()).filter(row => row.gameId === game);
  const clearings = (db = a) => Array.from(db.db.terrainClearing.iter()).filter(row => row.gameId === game);
  const build = (kind: string, x: number, z: number) => status(a, cb => a.reducers.onBuildRotated(cb), () => a.reducers.buildRotated(x, z, kind, 0));
  assert.equal((await build('housing', 25, 25)).tag, 'Committed');
  for (const kind of ['saw_pit', 'quarry', 'iron_mine']) {
    const failed = await build(kind, 45, 25);
    assert.equal(failed.tag, 'Failed', `${kind} must reject ordinary grass`);
    if (failed.tag === 'Failed') assert.match(failed.value, /Gelände/);
  }
  for (const point of [[-245, -139], [-155, -109], [-195, -275], [92, 0]]) {
    assert.equal((await build('granary', point[0], point[1])).tag, 'Failed', 'normal buildings reject rock, ore and water');
  }
  await until(() => clearings().length === 3, 'only the two start keeps and housing clear trees');
  assert.equal((await build('granary', -115, 55)).tag, 'Committed', 'ordinary building must allow forest');
  assert.equal((await build('stockpile', -199, 301)).tag, 'Committed', 'dunes must allow a level foundation');
  assert.equal((await build('saw_pit', -115, 85)).tag, 'Committed');
  assert.equal((await build('quarry', -245, -139)).tag, 'Committed');
  assert.equal((await build('iron_mine', -155, -109)).tag, 'Committed');
  await until(() => buildings(b).length === 8 && clearings(b).length === 8, 'other player receives start keeps, construction and tree clearance');
  const cleared = clearings().find(c => c.x === -115 && c.z === 55)!;
  assert.ok(cleared && cleared.halfX >= 3 && cleared.halfZ >= 3);
  const building = buildings().find(row => row.kind === 'granary')!;
  assert.equal((await status(a, cb => a.reducers.onRemoveBuilding(cb), () => a.reducers.removeBuilding(building.id))).tag, 'Committed');
  await until(() => buildings(b).length === 7, 'demolition arrives');
  assert.ok(clearings(b).some(c => c.id === cleared.id), 'trees stay cleared after demolition');
  a.disconnect();
  const c = await connect();
  assert.equal((await status(c, cb => c.reducers.onJoinGame(cb), () => c.reducers.joinGame(game))).tag, 'Committed');
  await until(() => clearings(c).some(row => row.id === cleared.id), 'tree clearance survives reconnect');
  console.log('PASS: resource placement, forest construction, dune foundations, failed-action rollback, multiplayer replication, demolition and reconnect persistence.');
} finally {
  for (const client of clients) client.disconnect();
}

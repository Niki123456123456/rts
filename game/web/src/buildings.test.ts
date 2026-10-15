// Two real players exercise authoritative construction, navigation and combat.
import assert from 'node:assert/strict';
import { DbConnection, type ReducerEventContext } from './module_bindings';
import { strongholdNetwork } from './network';
const uri = process.env.SPACETIMEDB_URI ?? 'http://127.0.0.1:3300';
const database = process.env.SPACETIMEDB_DATABASE ?? 'stronghold-test';
const clients: DbConnection[] = [];
const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
async function until(test: () => boolean, message: string, timeout = 15000) {
  const end = Date.now() + timeout;
  while (!test()) { assert.ok(Date.now() < end, message); await delay(20); }
}
async function connect() {
  return new Promise<DbConnection>((resolve, reject) => {
    const db = DbConnection.builder().withUri(uri).withModuleName(database)
      .onConnect(db => {
        clients.push(db);
        db.subscriptionBuilder().onApplied(() => resolve(db)).onError(ctx => reject(ctx.event))
          .subscribe(['SELECT * FROM game', 'SELECT * FROM participant', 'SELECT * FROM army_unit', 'SELECT * FROM building']);
      }).onConnectError((_ctx, err) => reject(err)).build();
    setTimeout(() => reject(new Error('Connection timeout')), 8000).unref();
    return db;
  });
}
function status(db: DbConnection, register: (cb: (ctx: ReducerEventContext) => void) => void, call: () => void) {
  return new Promise<string>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Reducer timeout')), 8000);
    let done = false;
    register(ctx => {
      if (done || ctx.event.callerConnectionId?.toHexString() !== db.connectionId.toHexString()) return;
      done = true; clearTimeout(timer); resolve(ctx.event.status.tag);
    }); call();
  });
}
function membership(db: DbConnection) {
  return Array.from(db.db.participant.iter()).find(p => p.connectionId.toHexString() === db.connectionId.toHexString());
}
try {
  const a = await connect(), b = await connect(), c = await connect();
  const title = `Buildings test ${Date.now()}`;
  assert.equal(await status(a, cb => a.reducers.onCreateGame(cb), () => a.reducers.createGame(title)), 'Committed');
  const game = membership(a)!.gameId;
  assert.equal(await status(b, cb => b.reducers.onJoinGame(cb), () => b.reducers.joinGame(game)), 'Committed');
  const buildings = () => Array.from(a.db.building.iter()).filter(x => x.gameId === game);
  const own = () => Array.from(a.db.armyUnit.iter()).filter(u => u.gameId === game && u.owner.toHexString() === a.connectionId.toHexString());
  await until(() => own().length === 20, 'Own army arrives');
  const assertSpacing = () => {
    const units = Array.from(a.db.armyUnit.iter()).filter(u => u.gameId === game);
    for (let i = 0; i < units.length; i++) for (let j = i + 1; j < units.length; j++) {
      assert.ok(Math.hypot(units[i].x - units[j].x, units[i].z - units[j].z) >= 0.5499, 'Units from either player never overlap');
    }
  };
  assertSpacing();
  a.db.armyUnit.onUpdate(assertSpacing);
  const build = (db: DbConnection, x: number, z: number) => status(db, cb => db.reducers.onBuildGranary(cb), () => db.reducers.buildGranary(x, z));
  const remove = (db: DbConnection, id: bigint) => status(db, cb => db.reducers.onRemoveBuilding(cb), () => db.reducers.removeBuilding(id));
  const attack = (db: DbConnection, ids: bigint[], id: bigint) => status(db, cb => db.reducers.onAttackBuilding(cb), () => db.reducers.attackBuilding(ids, id));
  assert.equal(await build(b, 0, 9), 'Committed');
  await until(() => buildings().length === 1, 'Both clients receive the building');
  const granary = buildings()[0];
  assert.equal(granary.health, 1); assert.equal(granary.kind, 'granary');
  assert.equal(granary.owner.toHexString(), b.connectionId.toHexString());
  assert.equal(await build(a, 5, 9), 'Failed', 'Overlapping placement rejected');
  assert.equal(await build(a, 38, 9), 'Failed', 'Out-of-map footprint rejected');
  assert.equal(await build(a, NaN, 9), 'Failed');
  assert.equal(await build(a, 0, 0), 'Failed', 'Cannot build over units');
  assert.equal(await build(a, 0, -12), 'Failed', 'Uneven terrain rejected');
  assert.equal(await remove(a, granary.id), 'Failed', 'Enemy demolition rejected');
  assert.equal(await attack(b, Array.from(b.db.armyUnit.iter()).filter(u => u.owner.toHexString() === b.connectionId.toHexString()).map(u => u.id), granary.id), 'Failed', 'Cannot attack own building');
  const unit = own()[0];
  const enemy = Array.from(a.db.armyUnit.iter()).find(u => u.owner.toHexString() === b.connectionId.toHexString())!;
  assert.equal(await attack(a, [unit.id, enemy.id], granary.id), 'Failed', 'Mixed ownership rejected atomically');
  assert.equal(await status(a, cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([unit.id], 0, 9)), 'Failed', 'Blocked target rejected');
  assert.equal(await status(a, cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([unit.id], enemy.x, enemy.z)), 'Committed');
  const allocated = a.db.armyUnit.id.find(unit.id)!;
  for (const peer of Array.from(a.db.armyUnit.iter()).filter(u => u.gameId === game && u.id !== unit.id)) {
    assert.ok(Math.hypot(allocated.targetX - peer.x, allocated.targetZ - peer.z) >= 0.5499, 'Occupied destinations are moved to free ground');
  }
  let walkedAround = false;
  let tracking = true;
  a.db.armyUnit.onUpdate((_ctx, _old, u) => {
    if (!tracking || u.id !== unit.id) return;
    assert.ok(!(Math.abs(u.x) < 2.73896 + 0.269 && Math.abs(u.z - 9) < 2.53584 + 0.269), 'Unit never crosses inflated collision footprint');
    if (u.z > 6.2 && u.z < 11.8 && Math.abs(u.x) > 3) walkedAround = true;
  });
  assert.equal(await status(a, cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([unit.id], 0, 16)), 'Committed');
  await until(() => a.db.armyUnit.id.find(unit.id)?.z === 16, 'Unit follows route around granary');
  assert.ok(walkedAround); tracking = false;
  assert.equal(b.db.armyUnit.id.find(unit.id)?.z, 16);

  // Another game may use the same coordinates, but cannot attack this game.
  assert.equal(await status(c, cb => c.reducers.onCreateGame(cb), () => c.reducers.createGame(`${title} isolated`)), 'Committed');
  assert.equal(await build(c, 0, 9), 'Committed');
  const cUnits = Array.from(c.db.armyUnit.iter()).filter(u => u.owner.toHexString() === c.connectionId.toHexString());
  assert.equal(await attack(c, cUnits.map(u => u.id), granary.id), 'Failed');

  let damaged = false;
  a.db.building.onUpdate((_ctx, old, row) => {
    if (row.id === granary.id) { assert.ok(row.health >= 0 && row.health < old.health); damaged = true; }
  });
  assert.equal(await attack(a, own().map(u => u.id), granary.id), 'Committed');
  const targets = own().map(u => [u.targetX, u.targetZ]);
  for (let i = 0; i < targets.length; i++) for (let j = i + 1; j < targets.length; j++) {
    assert.ok(Math.hypot(targets[i][0] - targets[j][0], targets[i][1] - targets[j][1]) >= 0.55, 'Attack destinations are distinct');
  }
  assert.ok(targets.some(([x]) => x < -3) && targets.some(([x]) => x > 3)
    && targets.some(([, z]) => z < 6) && targets.some(([, z]) => z > 12), 'Attackers assigned around all four walls');
  await until(() => !a.db.building.id.find(granary.id), 'Attack reduces health to zero and removes building');
  assert.ok(damaged, 'Damage was replicated');
  await until(() => !b.db.building.id.find(granary.id), 'Destruction replicated to owner');
  assert.equal(await build(a, 15, 14), 'Committed');
  const ownBuilding = buildings().find(x => x.owner.toHexString() === a.connectionId.toHexString())!;
  assert.equal(await remove(a, ownBuilding.id), 'Committed');
  assert.equal(await remove(a, ownBuilding.id), 'Failed');

  // Construction changes an already-running route. Every intermediate point
  // must avoid the newly inserted collision mesh.
  const runner = own()[0];
  assert.equal(await status(a, cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([runner.id], 20, 16)), 'Committed');
  assert.equal(await build(b, 12, 16), 'Committed');
  let rerouted = false;
  a.db.armyUnit.onUpdate((_ctx, _old, u) => {
    if (u.id !== runner.id || u.targetX !== 20) return;
    assert.ok(!(Math.abs(u.x - 12) < 3.007 && Math.abs(u.z - 16) < 2.804), 'Construction replans live movement');
    if (u.x > 9 && u.x < 15 && Math.abs(u.z - 16) > 2.8) rerouted = true;
  });
  await until(() => a.db.armyUnit.id.find(runner.id)?.x === 20, 'Rerouted unit reaches goal');
  assert.ok(rerouted);
  assert.equal(await remove(b, buildings().find(x => x.x === 12)!.id), 'Committed');

  // Exercise the browser bridge's building actions and scoped subscription.
  const send = (action: object) => strongholdNetwork.send(JSON.stringify(action));
  const snapshot = () => JSON.parse(strongholdNetwork.snapshot());
  send({ action: 'connect', uri, database }); await until(() => snapshot().ready, 'Bridge ready');
  send({ action: 'join', game_id: game.toString() }); await until(() => snapshot().units.length === 60, 'Bridge army joined');
  send({ action: 'build_granary', x: -27, z: 15 });
  await until(() => snapshot().buildings.some((b: {owned: boolean}) => b.owned), 'Bridge receives its building');
  const bridgeBuilding = snapshot().buildings.find((b: {owned: boolean}) => b.owned);
  assert.equal(bridgeBuilding.health, 1);
  send({ action: 'remove_building', building_id: bridgeBuilding.id });
  await until(() => snapshot().buildings.length === 0, 'Bridge removes building');
  send({ action: 'build_granary', x: -27, z: 15 }); await until(() => snapshot().buildings.length === 1, 'Bridge rebuilds');
  send({ action: 'leave' }); await until(() => snapshot().game_id === null && snapshot().buildings.length === 0, 'Leave clears building subscription');
  await until(() => buildings().length === 0, 'Leave removes owned buildings');
  assert.equal(await build(b, 0, 25), 'Committed');
  b.disconnect(); await until(() => buildings().length === 0, 'Disconnect removes owned buildings');
  console.log('Building integration passed: placement, overlap, ownership, shortest routes, live replanning, unit separation, surrounding attacks, attack damage, destruction, game isolation, browser bridge and cleanup.');
} finally {
  strongholdNetwork.disconnect();
  for (const db of clients) db.disconnect();
}

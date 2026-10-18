// Two real players exercise authoritative construction, navigation and combat.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
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
          .subscribe(['SELECT * FROM game', 'SELECT * FROM participant', 'SELECT * FROM army_unit', 'SELECT * FROM building', 'SELECT * FROM building_rotation']);
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
  const build = (db: DbConnection, x: number, z: number) => status(db, cb => db.reducers.onBuild(cb), () => db.reducers.build(x, z, 'granary'));
  const remove = (db: DbConnection, id: bigint) => status(db, cb => db.reducers.onRemoveBuilding(cb), () => db.reducers.removeBuilding(id));
  const attack = (db: DbConnection, ids: bigint[], id: bigint) => status(db, cb => db.reducers.onAttackBuilding(cb), () => db.reducers.attackBuilding(ids, id));
  assert.equal(await build(b, 1, 9), 'Committed');
  await until(() => buildings().length === 1, 'Both clients receive the building');
  const granary = buildings()[0];
  assert.equal(granary.health, 1); assert.equal(granary.kind, 'granary');
  assert.equal(granary.owner.toHexString(), b.connectionId.toHexString());
  assert.equal(await build(a, 5, 9), 'Failed', 'Overlapping placement rejected');
  assert.equal(await build(a, 398, 9), 'Failed', 'Out-of-map footprint rejected');
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
    assert.ok(!(Math.abs(u.x - granary.x) < 2.73896 + 0.269 && Math.abs(u.z - granary.z) < 2.53584 + 0.269), 'Unit never crosses inflated collision footprint');
    if (Math.abs(u.z - granary.z) < 2.8 && Math.abs(u.x - granary.x) > 3) walkedAround = true;
  });
  assert.equal(await status(a, cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([unit.id], 0, 16)), 'Committed');
  await until(() => a.db.armyUnit.id.find(unit.id)?.z === 16, 'Unit follows route around granary');
  assert.ok(walkedAround); tracking = false;
  assert.equal(b.db.armyUnit.id.find(unit.id)?.z, 16);

  // Barracks have a much larger footprint than granaries and retain their kind.
  const buildBarracks = (db: DbConnection, x: number, z: number) => status(db, cb => db.reducers.onBuild(cb), () => db.reducers.build(x, z, 'barracks'));
  assert.equal(await buildBarracks(b, 20, 22), 'Committed');
  await until(() => buildings().some(x => x.kind === 'barracks'), 'Both clients receive barracks');
  const barracks = buildings().find(x => x.kind === 'barracks')!;
  assert.equal(await build(a, 26, 22), 'Failed', 'Granary cannot overlap the larger barracks footprint');
  assert.equal(await buildBarracks(a, 396, 22), 'Failed', 'Full barracks footprint must fit within the map');
  assert.equal(await buildBarracks(a, 0, 0), 'Failed', 'Barracks cannot cover units');
  assert.equal(await buildBarracks(a, 0, -12), 'Failed', 'Barracks require level terrain');
  assert.equal(await status(a, cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([unit.id], 25, 22)), 'Failed', 'Movement targets inside barracks footprint are blocked');
  assert.equal(await attack(a, [unit.id], barracks.id), 'Committed');
  const attacker = a.db.armyUnit.id.find(unit.id)!;
  const edgeDistance = Math.hypot(Math.max(0, Math.abs(attacker.targetX - barracks.x) - 6.157582), Math.max(0, Math.abs(attacker.targetZ - barracks.z) - 5.137907));
  assert.ok(Math.abs(edgeDistance - 0.6) < 0.001, 'Attack slots follow the barracks walls');
  assert.equal(await remove(a, barracks.id), 'Failed', 'Enemy barracks cannot be removed');
  assert.equal(await remove(b, barracks.id), 'Committed');

  // Stockpiles have their own wider footprint, including overlap and attack checks.
  const buildStockpile = (db: DbConnection, x: number, z: number) => status(db, cb => db.reducers.onBuild(cb), () => db.reducers.build(x, z, 'stockpile'));
  assert.equal(await buildStockpile(b, -25, 22), 'Committed');
  await until(() => buildings().some(x => x.kind === 'stockpile'), 'Both clients receive stockpile');
  const stockpile = buildings().find(x => x.kind === 'stockpile')!;
  assert.equal(stockpile.health, 1);
  assert.equal(await build(a, -16, 22), 'Failed', 'Granary cannot overlap the wider stockpile footprint');
  assert.equal(await buildStockpile(a, 396, 22), 'Failed', 'Full stockpile footprint must fit within the map');
  assert.equal(await buildStockpile(a, 0, 0), 'Failed', 'Stockpile cannot cover units');
  assert.equal(await buildStockpile(a, 0, -12), 'Failed', 'Stockpile requires level terrain');
  assert.equal(await status(a, cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([unit.id], -18.1, 22)), 'Failed', 'Movement targets inside stockpile footprint are blocked');
  assert.equal(await attack(a, [unit.id], stockpile.id), 'Committed');
  const stockpileAttacker = a.db.armyUnit.id.find(unit.id)!;
  const stockpileEdgeDistance = Math.hypot(Math.max(0, Math.abs(stockpileAttacker.targetX - stockpile.x) - 7.217365), Math.max(0, Math.abs(stockpileAttacker.targetZ - stockpile.z) - 5.355486));
  assert.ok(Math.abs(stockpileEdgeDistance - 0.6) < 0.001, 'Attack slots follow stockpile walls');
  assert.equal(await remove(a, stockpile.id), 'Failed', 'Enemy stockpile cannot be removed');
  assert.equal(await remove(b, stockpile.id), 'Committed');

  // The generic build command validates names and all fortification kinds.
  await until(() => !buildings().some(row => row.id === stockpile.id), 'Stockpile removal replicated before checking unchanged building count');
  const beforeUnknown = buildings().length;
  assert.equal(await status(a, cb => a.reducers.onBuild(cb), () => a.reducers.build(80, 80, 'unknown')), 'Failed');
  assert.equal(buildings().length, beforeUnknown, 'Unknown types do not create buildings');
  const catalog = JSON.parse(readFileSync(new URL('../../building-models.json', import.meta.url), 'utf8')) as {id: string; category: string; grid: [number, number]}[];
  const fortifications = catalog.filter(x => x.category === 'fortifications');
  assert.equal(fortifications.length, 18);
  for (const [index, entry] of fortifications.entries()) {
    const x = -250 + index % 6 * 70;
    const z = 140 + Math.floor(index / 6) * 60;
    assert.equal(await status(b, cb => b.reducers.onBuild(cb), () => b.reducers.build(x, z, entry.id)), 'Committed', entry.id);
    await until(() => buildings().some(row => row.kind === entry.id), `Replicated ${entry.id}`);
    const row = buildings().find(row => row.kind === entry.id)!;
    assert.ok(Number.isInteger((row.x - entry.grid[0]) / 2), `${entry.id}: X edges follow grid lines`);
    assert.ok(Number.isInteger((row.z - entry.grid[1]) / 2), `${entry.id}: Z edges follow grid lines`);
    assert.equal(await build(a, x, z), 'Failed', `${entry.id} blocks overlapping buildings`);
    assert.equal(await status(a, cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([unit.id], row.x, row.z)), 'Failed', `${entry.id} blocks movement targets`);
    assert.equal(await attack(a, [unit.id], row.id), 'Committed', `Reachable attack positions for ${entry.id}`);
    assert.equal(await remove(b, row.id), 'Committed');
  }
  // Two stair towers touch at one grid edge despite overlapping collision meshes.
  const stair = (x: number, z: number) => status(b, cb => b.reducers.onBuild(cb), () => b.reducers.build(x, z, 'tower_stairwell'));
  assert.equal(await stair(100.8, 100.7), 'Committed');
  assert.equal(await stair(104.3, 100.4), 'Committed', 'Adjacent tower must be allowed despite mesh overlap');
  await until(() => buildings().filter(row => row.kind === 'tower_stairwell').length === 2, 'Adjacent towers replicated');
  const towers = buildings().filter(row => row.kind === 'tower_stairwell').sort((a, b) => a.x - b.x);
  assert.deepEqual(towers.map(row => [row.x, row.z]), [[100, 100], [104, 100]]);
  assert.equal(towers[1].x + 2 - (towers[0].x - 2), 8, 'Two stair towers occupy 8 by 4 metres');
  assert.equal(await stair(102.1, 100.3), 'Failed', 'Shared occupied grid cells are rejected');
  assert.equal(await stair(100.1, 99.8), 'Failed', 'Coordinates snapping to an occupied plot are rejected');
  assert.equal(await remove(b, towers[0].id), 'Committed');
  assert.equal(await stair(102.1, 100.3), 'Failed', 'The remaining tower still reserves its cells');
  assert.equal(await remove(b, towers[1].id), 'Committed');
  assert.equal(await stair(102.1, 100.3), 'Committed', 'Removing towers frees their grid cells');
  await until(() => buildings().some(row => row.kind === 'tower_stairwell' && row.x === 102), 'Replacement tower replicated');
  assert.equal(await remove(b, buildings().find(row => row.kind === 'tower_stairwell' && row.x === 102)!.id), 'Committed');

  // Persist all four orientations; 90° swaps a 12×10 m barracks plot to 10×12 m.
  const buildRotated = (x: number, z: number, kind: string, rotation: number) => status(b, cb => b.reducers.onBuildRotated(cb), () => b.reducers.buildRotated(x, z, kind, rotation));
  assert.equal(await buildRotated(100, 100, 'barracks', 4), 'Failed', 'Invalid rotations are rejected');
  for (let rotation = 0; rotation < 4; rotation++) {
    const x = 100 + rotation * 40 + (rotation % 2);
    const z = rotation % 2 === 0 ? 101 : 100;
    assert.equal(await buildRotated(100.3 + rotation * 40, 100.3, 'barracks', rotation), 'Committed');
    await until(() => buildings().some(row => row.kind === 'barracks' && row.x === x && row.z === z), 'Rotated building replicated at its snapped centre');
    const turned = buildings().find(row => row.kind === 'barracks' && row.x === x && row.z === z)!;
    await until(() => a.db.buildingRotation.buildingId.find(turned.id)?.quarterTurns === rotation, 'Orientation replicated to second player');
    assert.equal(b.db.buildingRotation.buildingId.find(turned.id)?.quarterTurns, rotation);
    if (rotation === 1) {
      assert.equal(await status(a, cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([unit.id], x, z + 5.8)), 'Failed', 'Pathfinding uses rotated collision dimensions');
      assert.equal(await attack(a, [unit.id], turned.id), 'Committed');
      const attacker = a.db.armyUnit.id.find(unit.id)!;
      const edge = Math.hypot(Math.max(0, Math.abs(attacker.targetX - x) - 5.137907), Math.max(0, Math.abs(attacker.targetZ - z) - 6.157582));
      assert.ok(Math.abs(edge - 0.6) < 0.001, 'Attack positions follow rotated walls');
      assert.equal(await buildRotated(x + 7, z, 'tower_stairwell', 0), 'Committed', 'Adjacent plot allowed by swapped raster dimensions');
      await until(() => buildings().some(row => row.kind === 'tower_stairwell' && row.x === x + 7 && row.z === z), 'Rotated neighbour replicated');
      assert.equal(await remove(b, buildings().find(row => row.kind === 'tower_stairwell' && row.x === x + 7)!.id), 'Committed');
      assert.equal(await buildRotated(x + 5, z, 'tower_stairwell', 0), 'Failed', 'Occupied rotated cells cannot be reused');
    }
    assert.equal(await remove(b, turned.id), 'Committed');
    await until(() => !a.db.buildingRotation.buildingId.find(turned.id), 'Demolition removes orientation metadata');
  }

  assert.equal(await status(a, cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([unit.id], 120, 120)), 'Committed', 'Movement orders beyond the old map limits are accepted');
  assert.equal(await status(a, cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([unit.id], unit.x, unit.z)), 'Committed');

  // Another game may use the same coordinates, but cannot attack this game.
  assert.equal(await status(c, cb => c.reducers.onCreateGame(cb), () => c.reducers.createGame(`${title} isolated`)), 'Committed');
  assert.equal(await build(c, 1, 9), 'Committed');
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
  assert.ok(targets.some(([x]) => x < granary.x - 3) && targets.some(([x]) => x > granary.x + 3)
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
  await until(() => buildings().some(row => row.x === 13 && row.z === 17), 'Snapped building replicated before tracking rerouting');
  const routeBuilding = buildings().find(row => row.x === 13 && row.z === 17)!;
  let rerouted = false;
  a.db.armyUnit.onUpdate((_ctx, _old, u) => {
    if (u.id !== runner.id || u.targetX !== 20) return;
    assert.ok(!(Math.abs(u.x - routeBuilding.x) < 3.007 && Math.abs(u.z - routeBuilding.z) < 2.804), 'Construction replans live movement');
    if (Math.abs(u.x - routeBuilding.x) < 3 && Math.abs(u.z - routeBuilding.z) > 2.8) rerouted = true;
  });
  await until(() => a.db.armyUnit.id.find(runner.id)?.x === 20, 'Rerouted unit reaches goal');
  assert.ok(rerouted);
  assert.equal(await remove(b, routeBuilding.id), 'Committed');

  // Exercise the browser bridge's building actions and scoped subscription.
  const send = (action: object) => strongholdNetwork.send(JSON.stringify(action));
  const snapshot = () => JSON.parse(strongholdNetwork.snapshot());
  send({ action: 'connect', uri, database }); await until(() => snapshot().ready, 'Bridge ready');
  send({ action: 'join', game_id: game.toString() }); await until(() => snapshot().units.length === 60, 'Bridge army joined');
  send({ action: 'build', building: 'barracks', x: -27, z: 15, rotation: 1 });
  await until(() => snapshot().buildings.some((b: {owned: boolean; rotation: number}) => b.owned && b.rotation === 1), 'Bridge receives its rotated building');
  const bridgeBuilding = snapshot().buildings.find((b: {owned: boolean}) => b.owned);
  assert.equal(bridgeBuilding.health, 1);
  assert.equal(bridgeBuilding.kind, 'barracks');
  assert.equal(bridgeBuilding.rotation, 1);
  assert.deepEqual([bridgeBuilding.x, bridgeBuilding.z], [-27, 16]);
  send({ action: 'remove_building', building_id: bridgeBuilding.id });
  await until(() => snapshot().buildings.length === 0, 'Bridge removes building');
  send({ action: 'build', building: 'stockpile', x: -27, z: 15 });
  await until(() => snapshot().buildings.some((b: {kind: string}) => b.kind === 'stockpile'), 'Bridge receives stockpile with its building kind');
  const bridgeStockpile = snapshot().buildings.find((b: {kind: string}) => b.kind === 'stockpile');
  assert.equal(bridgeStockpile.health, 1);
  send({ action: 'remove_building', building_id: bridgeStockpile.id });
  await until(() => snapshot().buildings.length === 0, 'Bridge removes stockpile');
  send({ action: 'build', building: 'granary', x: -27, z: 15 }); await until(() => snapshot().buildings.length === 1, 'Bridge rebuilds');
  send({ action: 'leave' }); await until(() => snapshot().game_id === null && snapshot().buildings.length === 0, 'Leave clears building subscription');
  await until(() => buildings().length === 0, 'Leave removes owned buildings');
  assert.equal(await build(b, 0, 25), 'Committed');
  b.disconnect(); await until(() => buildings().length === 0, 'Disconnect removes owned buildings');
  await until(() => !Array.from(a.db.buildingRotation.iter()).some(row => row.gameId === game), 'Leave and disconnect remove orientation metadata');
  console.log('Building integration passed: placement, overlap, ownership, shortest routes, live replanning, unit separation, surrounding attacks, attack damage, destruction, game isolation, browser bridge and cleanup.');
} finally {
  strongholdNetwork.disconnect();
  for (const db of clients) db.disconnect();
}

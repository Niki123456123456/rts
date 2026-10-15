// Runs against a published, disposable database. No browser or renderer required.
import assert from 'node:assert/strict';
import { DbConnection, type ReducerEventContext } from './module_bindings';
import { strongholdNetwork } from './network';

const uri = process.env.SPACETIMEDB_URI ?? 'http://127.0.0.1:3300';
const database = process.env.SPACETIMEDB_DATABASE ?? 'stronghold-test';
const clients: DbConnection[] = [];
const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
async function until(predicate: () => boolean, message: string) {
  const deadline = Date.now() + 8000;
  while (!predicate()) {
    assert.ok(Date.now() < deadline, message);
    await delay(20);
  }
}
async function connect() {
  return new Promise<DbConnection>((resolve, reject) => {
    const conn = DbConnection.builder().withUri(uri).withModuleName(database)
      .onConnect(db => {
        clients.push(db);
        db.subscriptionBuilder().onApplied(() => resolve(db))
          .onError(ctx => reject(ctx.event))
          .subscribe(['SELECT * FROM game', 'SELECT * FROM participant', 'SELECT * FROM army_unit']);
      })
      .onConnectError((_ctx, err) => reject(err)).build();
    setTimeout(() => reject(new Error('Connection timed out')), 8000).unref();
    return conn;
  });
}
function status(register: (cb: (ctx: ReducerEventContext) => void) => void, call: () => void) {
  return new Promise<ReducerEventContext['event']['status']>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Reducer timed out')), 8000);
    let done = false;
    register(ctx => {
      if (done) return;
      done = true;
      clearTimeout(timer);
      resolve(ctx.event.status);
    });
    call();
  });
}
function membership(db: DbConnection) {
  return Array.from(db.db.participant.iter()).find(p => p.connectionId.toHexString() === db.connectionId.toHexString());
}

try {
  const a = await connect();
  const b = await connect();
  const title = `Multiplayer test ${Date.now()}`;
  assert.equal((await status(cb => a.reducers.onCreateGame(cb), () => a.reducers.createGame(title))).tag, 'Committed');
  await until(() => !!membership(a) && Array.from(b.db.game.iter()).some(g => g.title === title), 'Game must appear on both clients');
  const gameId = membership(a)!.gameId;
  const gameUnits = () => Array.from(a.db.armyUnit.iter()).filter(u => u.gameId === gameId);
  assert.equal(gameUnits().length, 20);
  assert.ok(gameUnits().every(u => u.health === 1 && u.owner.toHexString() === a.connectionId.toHexString()));
  const aColor = membership(a)!.color;
  assert.equal((await status(cb => b.reducers.onJoinGame(cb), () => b.reducers.joinGame(gameId))).tag, 'Committed');
  await until(() => membership(b)?.gameId === gameId, 'Second client joins the same game');
  await until(() => gameUnits().length === 40, 'Each player gets 20 units');
  assert.notEqual(membership(b)!.color, aColor);
  const ownUnits = () => gameUnits().filter(u => u.owner.toHexString() === a.connectionId.toHexString());
  assert.equal(ownUnits().length, 20);
  assert.ok(ownUnits().every(u => u.color === aColor && u.health === 1));
  const unit = ownUnits()[0];
  const enemy = gameUnits().find(u => u.owner.toHexString() === b.connectionId.toHexString())!;
  assert.equal((await status(cb => b.reducers.onSetTargets(cb), () => b.reducers.setTargets([unit.id], 5, 5))).tag, 'Failed');
  assert.equal((await status(cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([unit.id, enemy.id], 5, 5))).tag, 'Failed');
  assert.equal(a.db.armyUnit.id.find(unit.id)!.targetX, unit.x);
  assert.equal((await status(cb => a.reducers.onJoinGame(cb), () => a.reducers.joinGame(gameId))).tag, 'Committed');
  assert.equal(ownUnits().length, 20, 'Joining twice must not duplicate an army');
  const targetX = unit.x + 1;
  assert.equal((await status(cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([unit.id], targetX, unit.z))).tag, 'Committed');
  await until(() => a.db.armyUnit.id.find(unit.id)?.x === targetX, 'Server movement must reach target');
  await until(() => b.db.armyUnit.id.find(unit.id)?.x === targetX, 'Both clients receive server movement');
  assert.equal(a.db.armyUnit.id.find(unit.id)?.z, unit.z);

  const pair = ownUnits().slice(0, 2);
  const centerX = (pair[0].x + pair[1].x) / 2;
  const centerZ = (pair[0].z + pair[1].z) / 2;
  assert.equal((await status(cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets(pair.map(u => u.id), 2, 2))).tag, 'Committed');
  for (const original of pair) {
    const updated = a.db.armyUnit.id.find(original.id)!;
    assert.ok(Math.abs(updated.targetX - (2 + original.x - centerX)) < 0.00001);
    assert.ok(Math.abs(updated.targetZ - (2 + original.z - centerZ)) < 0.00001);
  }

  assert.equal((await status(cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([unit.id], NaN, 0))).tag, 'Failed');
  assert.equal((await status(cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([unit.id], 1000, 0))).tag, 'Failed');
  assert.equal((await status(cb => a.reducers.onCreateGame(cb), () => a.reducers.createGame('   '))).tag, 'Failed');
  assert.equal((await status(cb => a.reducers.onCreateGame(cb), () => a.reducers.createGame(title))).tag, 'Failed');

  // Client B changes games; orders against the previous game must be rejected.
  assert.equal((await status(cb => b.reducers.onCreateGame(cb), () => b.reducers.createGame(`${title} other`))).tag, 'Committed');
  await until(() => membership(b)?.gameId !== gameId && gameUnits().length === 20, 'Switch games removes the previous army');
  assert.equal((await status(cb => b.reducers.onSetTargets(cb), () => b.reducers.setTargets([unit.id], 5, 5))).tag, 'Failed');

  // A forged call cannot speed up or teleport units by running the scheduler.
  assert.equal((await status(cb => a.reducers.onMoveUnits(cb), () => a.reducers.moveUnits({ scheduledId: 1n, scheduledAt: { tag: 'Interval', value: 50000n } } as never))).tag, 'Failed');

  assert.equal((await status(cb => a.reducers.onLeaveGame(cb), () => a.reducers.leaveGame())).tag, 'Committed');
  await until(() => !membership(a) && gameUnits().length === 0, 'Leave removes membership and army');
  assert.equal((await status(cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([unit.id], 5, 5))).tag, 'Failed');
  const bId = b.connectionId.toHexString();
  b.disconnect();
  await until(() => !Array.from(a.db.participant.iter()).some(p => p.connectionId.toHexString() === bId), 'Disconnect removes membership');

  // Exercise the exact browser/WASM bridge, including scoped subscriptions.
  const bridgeSend = (action: object) => strongholdNetwork.send(JSON.stringify(action));
  const snapshot = () => JSON.parse(strongholdNetwork.snapshot());
  bridgeSend({ action: 'connect', uri, database });
  await until(() => snapshot().ready, 'Browser bridge connects and subscribes');
  assert.equal(snapshot().units.length, 0);
  bridgeSend({ action: 'join', game_id: gameId.toString() });
  await until(() => snapshot().units.length === 20, 'Bridge subscribes to the joined game');
  assert.equal(snapshot().game_id, gameId.toString());
  const bridgeUnit = snapshot().units[0];
  assert.ok(snapshot().units.every((u: {owned: boolean; health: number}) => u.owned && u.health === 1));
  bridgeSend({ action: 'targets', unit_ids: [bridgeUnit.id], x: 3, z: 3 });
  await until(() => snapshot().units.find((u: { id: string; x: number }) => u.id === bridgeUnit.id)?.x === 3, 'Bridge receives authoritative movement');
  bridgeSend({ action: 'leave' });
  await until(() => snapshot().game_id === null && snapshot().units.length === 0, 'Bridge clears units when leaving');
  bridgeSend({ action: 'connect', uri, database });
  await until(() => snapshot().ready, 'Bridge reconnects');
  assert.equal(snapshot().game_id, null);
  console.log('PASS: live lobby, 20 owned units/player, unique colors, health, enemy-order rejection, two-client movement, formations, game isolation, invalid orders, scheduler protection, leave, disconnect and browser bridge.');
} finally {
  strongholdNetwork.disconnect();
  for (const client of clients) client.disconnect();
}

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
          .subscribe(['SELECT * FROM game', 'SELECT * FROM player', 'SELECT * FROM unit']);
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
  return Array.from(db.db.player.iter()).find(p => p.connectionId.toHexString() === db.connectionId.toHexString());
}

try {
  const a = await connect();
  const b = await connect();
  const title = `Multiplayer test ${Date.now()}`;
  assert.equal((await status(cb => a.reducers.onCreateGame(cb), () => a.reducers.createGame(title))).tag, 'Committed');
  await until(() => !!membership(a) && Array.from(b.db.game.iter()).some(g => g.title === title), 'Game must appear on both clients');
  const gameId = membership(a)!.gameId;
  const gameUnits = () => Array.from(a.db.unit.iter()).filter(u => u.gameId === gameId);
  assert.equal(gameUnits().length, 100);
  assert.equal((await status(cb => b.reducers.onJoinGame(cb), () => b.reducers.joinGame(gameId))).tag, 'Committed');
  await until(() => membership(b)?.gameId === gameId, 'Second client joins the same game');
  const unit = gameUnits()[0];
  const targetX = unit.x + 1;
  assert.equal((await status(cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([unit.id], targetX, unit.z))).tag, 'Committed');
  await until(() => a.db.unit.id.find(unit.id)?.x === targetX, 'Server movement must reach target');
  await until(() => b.db.unit.id.find(unit.id)?.x === targetX, 'Both clients receive server movement');
  assert.equal(a.db.unit.id.find(unit.id)?.z, unit.z);

  const pair = gameUnits().slice(0, 2);
  const centerX = (pair[0].x + pair[1].x) / 2;
  const centerZ = (pair[0].z + pair[1].z) / 2;
  assert.equal((await status(cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets(pair.map(u => u.id), 2, 2))).tag, 'Committed');
  for (const original of pair) {
    const updated = a.db.unit.id.find(original.id)!;
    assert.ok(Math.abs(updated.targetX - (2 + original.x - centerX)) < 0.00001);
    assert.ok(Math.abs(updated.targetZ - (2 + original.z - centerZ)) < 0.00001);
  }

  assert.equal((await status(cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([unit.id], NaN, 0))).tag, 'Failed');
  assert.equal((await status(cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([unit.id], 1000, 0))).tag, 'Failed');
  assert.equal((await status(cb => a.reducers.onCreateGame(cb), () => a.reducers.createGame('   '))).tag, 'Failed');
  assert.equal((await status(cb => a.reducers.onCreateGame(cb), () => a.reducers.createGame(title))).tag, 'Failed');

  // Client B changes games; orders against the previous game must be rejected.
  assert.equal((await status(cb => b.reducers.onCreateGame(cb), () => b.reducers.createGame(`${title} other`))).tag, 'Committed');
  await until(() => membership(b)?.gameId !== gameId, 'Switch games');
  assert.equal((await status(cb => b.reducers.onSetTargets(cb), () => b.reducers.setTargets([unit.id], 5, 5))).tag, 'Failed');

  // A forged call cannot speed up or teleport units by running the scheduler.
  assert.equal((await status(cb => a.reducers.onMoveUnits(cb), () => a.reducers.moveUnits({ scheduledId: 1n, scheduledAt: { tag: 'Interval', value: 50000n } } as never))).tag, 'Failed');

  assert.equal((await status(cb => a.reducers.onLeaveGame(cb), () => a.reducers.leaveGame())).tag, 'Committed');
  await until(() => !membership(a), 'Leave removes membership');
  assert.equal((await status(cb => a.reducers.onSetTargets(cb), () => a.reducers.setTargets([unit.id], 5, 5))).tag, 'Failed');
  const bId = b.connectionId.toHexString();
  b.disconnect();
  await until(() => !Array.from(a.db.player.iter()).some(p => p.connectionId.toHexString() === bId), 'Disconnect removes membership');

  // Exercise the exact browser/WASM bridge, including scoped subscriptions.
  const bridgeSend = (action: object) => strongholdNetwork.send(JSON.stringify(action));
  const snapshot = () => JSON.parse(strongholdNetwork.snapshot());
  bridgeSend({ action: 'connect', uri, database });
  await until(() => snapshot().ready, 'Browser bridge connects and subscribes');
  assert.equal(snapshot().units.length, 0);
  bridgeSend({ action: 'join', game_id: gameId.toString() });
  await until(() => snapshot().units.length === 100, 'Bridge subscribes to the joined game');
  assert.equal(snapshot().game_id, gameId.toString());
  bridgeSend({ action: 'targets', unit_ids: [unit.id.toString()], x: 3, z: 3 });
  await until(() => snapshot().units.find((u: { id: string; x: number }) => u.id === unit.id.toString())?.x === 3, 'Bridge receives authoritative movement');
  bridgeSend({ action: 'leave' });
  await until(() => snapshot().game_id === null && snapshot().units.length === 0, 'Bridge clears units when leaving');
  bridgeSend({ action: 'connect', uri, database });
  await until(() => snapshot().ready, 'Bridge reconnects');
  assert.equal(snapshot().game_id, null);
  console.log('PASS: live lobby, 100 units/game, two-client movement, formations, game isolation, invalid orders, scheduler protection, leave, disconnect and browser bridge.');
} finally {
  strongholdNetwork.disconnect();
  for (const client of clients) client.disconnect();
}

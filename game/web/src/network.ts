import { DbConnection, type ReducerEventContext } from './module_bindings';

type Action =
  | { action: 'connect'; uri: string; database: string }
  | { action: 'create'; title: string }
  | { action: 'join'; game_id: string }
  | { action: 'leave' }
  | { action: 'build_granary'; x: number; z: number }
  | { action: 'remove_building'; building_id: string }
  | { action: 'attack_building'; unit_ids: string[]; building_id: string }
  | { action: 'formation_targets'; unit_ids: string[]; start_x: number; start_z: number; end_x: number; end_z: number }
  | { action: 'targets'; unit_ids: string[]; x: number; z: number };

let conn: DbConnection | undefined;
let connected = false;
let ready = false;
let error: string | null = null;
let activeGame: bigint | undefined;
let unitSubscription: ReturnType<ReturnType<DbConnection['subscriptionBuilder']>['subscribe']> | undefined;

function result(ctx: ReducerEventContext) {
  if (ctx.event.callerConnectionId?.toHexString() !== conn?.connectionId.toHexString()) return;
  const status = ctx.event.status;
  error = status.tag === 'Failed' ? status.value : status.tag === 'OutOfEnergy' ? 'Server is out of energy.' : null;
}

function connect(uri: string, database: string) {
  // SDK 1.1 otherwise downgrades https URLs to plain ws, which cloud hosts reject.
  const socketUri = new URL(uri);
  if (socketUri.protocol === 'https:') socketUri.protocol = 'wss:';
  if (socketUri.protocol === 'http:') socketUri.protocol = 'ws:';
  // Ignore callbacks from a connection replaced by a reconnect.
  conn?.disconnect();
  connected = false;
  ready = false;
  activeGame = undefined;
  unitSubscription = undefined;
  const connection = DbConnection.builder()
    .withUri(socketUri.toString())
    .withModuleName(database)
    .onConnect((db) => {
      if (conn !== db) return;
      connected = true;
      db.subscriptionBuilder()
        .onApplied(() => { if (conn === db) ready = true; })
        .onError((ctx) => { if (conn === db) error = String(ctx.event); })
        .subscribe(['SELECT * FROM game', 'SELECT * FROM participant']);
    })
    .onConnectError((_ctx, err) => {
      if (conn !== connection) return;
      connected = ready = false;
      error = String(err);
    })
    .onDisconnect((_ctx, err) => {
      if (conn !== connection) return;
      connected = ready = false;
      error = err ? String(err) : 'Disconnected from server.';
    })
    .build();
  conn = connection;
  connection.reducers.onCreateGame(result);
  connection.reducers.onJoinGame(result);
  connection.reducers.onLeaveGame(result);
  connection.reducers.onSetTargets(result);
  connection.reducers.onSetFormationTargets(result);
  connection.reducers.onBuildGranary(result);
  connection.reducers.onRemoveBuilding(result);
  connection.reducers.onAttackBuilding(result);
}

export const strongholdNetwork = {
  disconnect() {
    const previous = conn;
    conn = undefined;
    connected = ready = false;
    activeGame = undefined;
    unitSubscription = undefined;
    previous?.disconnect();
  },
  send(json: string) {
    const action: Action = JSON.parse(json);
    error = null;
    if (action.action === 'connect') { connect(action.uri, action.database); return; }
    if (!conn || !ready || !connected) throw new Error('Connect to a server first.');
    switch (action.action) {
      case 'create': conn.reducers.createGame(action.title); break;
      case 'join': conn.reducers.joinGame(BigInt(action.game_id)); break;
      case 'leave': conn.reducers.leaveGame(); break;
      case 'build_granary': conn.reducers.buildGranary(action.x, action.z); break;
      case 'remove_building': conn.reducers.removeBuilding(BigInt(action.building_id)); break;
      case 'attack_building': conn.reducers.attackBuilding(action.unit_ids.map(BigInt), BigInt(action.building_id)); break;
      case 'formation_targets': conn.reducers.setFormationTargets(action.unit_ids.map(BigInt), action.start_x, action.start_z, action.end_x, action.end_z); break;
      case 'targets': conn.reducers.setTargets(action.unit_ids.map(BigInt), action.x, action.z); break;
    }
  },
  snapshot() {
    const player = connected && ready && conn ? Array.from(conn.db.participant.iter())
      .find(p => p.connectionId.toHexString() === conn!.connectionId.toHexString()) : undefined;
    const gameId = player?.gameId;
    if (conn && connected && ready && gameId !== activeGame) {
      unitSubscription?.unsubscribe();
      activeGame = gameId;
      unitSubscription = gameId === undefined ? undefined : conn.subscriptionBuilder()
        .onError((ctx) => { error = String(ctx.event); })
        .subscribe([`SELECT * FROM army_unit WHERE game_id = ${gameId}`, `SELECT * FROM building WHERE game_id = ${gameId}`]);
    }
    return JSON.stringify({
      connected, ready, error, game_id: gameId === undefined ? null : gameId.toString(),
      games: ready && connected && conn ? Array.from(conn.db.game.iter())
        .sort((a, b) => a.id < b.id ? -1 : 1)
        .map(g => ({ id: g.id.toString(), title: g.title })) : [],
      buildings: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.building.iter())
        .filter(b => b.gameId === gameId)
        .map(b => ({ id: b.id.toString(), color: b.color, health: b.health, owned: b.owner.toHexString() === conn!.connectionId.toHexString(), x: b.x, y: b.y, z: b.z })) : [],
      units: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.armyUnit.iter())
        .filter(u => u.gameId === gameId)
        .map(u => ({ id: u.id.toString(), number: u.number, color: u.color, health: u.health, owned: u.owner.toHexString() === conn!.connectionId.toHexString(), x: u.x, y: u.y, z: u.z })) : [],
    });
  },
};

Object.assign(globalThis, { strongholdNetwork });

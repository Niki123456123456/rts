import { DbConnection, type ReducerEventContext } from './module_bindings';

type Action =
  | { action: 'connect'; uri: string; database: string }
  | { action: 'create'; title: string }
  | { action: 'join'; game_id: string }
  | { action: 'leave' }
  | { action: 'build'; x: number; z: number; building: string; rotation?: number }
  | { action: 'build_wall'; start_x: number; start_z: number; end_x: number; end_z: number }
  | { action: 'remove_building'; building_id: string }
  | { action: 'move_to_roof'; unit_ids: string[]; building_id: string; x: number; z: number }
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
  connection.reducers.onBuild(result);
  connection.reducers.onBuildRotated(result);
  connection.reducers.onBuildWall(result);
  connection.reducers.onMoveToRoof(result);
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
      case 'build': conn.reducers.buildRotated(action.x, action.z, action.building, action.rotation ?? 0); break;
      case 'build_wall': conn.reducers.buildWall(action.start_x, action.start_z, action.end_x, action.end_z); break;
      case 'remove_building': conn.reducers.removeBuilding(BigInt(action.building_id)); break;
      case 'move_to_roof': conn.reducers.moveToRoof(action.unit_ids.map(BigInt), BigInt(action.building_id), action.x, action.z); break;
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
        .subscribe([`SELECT * FROM army_unit WHERE game_id = ${gameId}`, `SELECT * FROM building WHERE game_id = ${gameId}`, `SELECT * FROM building_rotation WHERE game_id = ${gameId}`, `SELECT * FROM tower_occupant WHERE game_id = ${gameId}`, `SELECT * FROM player_resources WHERE game_id = ${gameId}`, `SELECT * FROM warehouse_stock WHERE game_id = ${gameId}`, `SELECT * FROM worker WHERE game_id = ${gameId}`, `SELECT * FROM producer WHERE game_id = ${gameId}`]);
    }
    return JSON.stringify({
      connected, ready, error, game_id: gameId === undefined ? null : gameId.toString(),
      resources: ready && connected && conn && gameId !== undefined ? (() => {
        const stock=conn.db.playerResources.connectionId.find(conn.connectionId);
        return stock?.gameId===gameId ? {wood:stock.wood,stone:stock.stone,iron:stock.iron} : null;
      })() : null,
      games: ready && connected && conn ? Array.from(conn.db.game.iter())
        .sort((a, b) => a.id < b.id ? -1 : 1)
        .map(g => ({ id: g.id.toString(), title: g.title })) : [],
      warehouses: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.warehouseStock.iter()).filter(s=>s.gameId===gameId).map(s=>({building_id:s.buildingId.toString(),wood:s.wood,stone:s.stone,iron:s.iron})) : [],
      workers: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.worker.iter()).filter(w=>w.gameId===gameId).map(w=>({unit_id:w.unitId.toString(),source_building:w.sourceBuilding.toString(),resource:w.resource,cargo:w.cargo})) : [],
      producers: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.producer.iter()).filter(p=>p.gameId===gameId).map(p=>({building_id:p.buildingId.toString(),resource:p.resource,buffer:p.buffer})) : [],
      roof_occupants: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.towerOccupant.iter())
        .filter(o => o.gameId === gameId)
        .map(o => ({ unit_id: o.unitId.toString(), building_id: o.buildingId.toString(), slot: o.slot, next_building: o.nextBuilding?.toString() ?? null, next_slot: o.nextSlot })) : [],
      buildings: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.building.iter())
        .filter(b => b.gameId === gameId)
        .map(b => ({ rotation: conn!.db.buildingRotation.buildingId.find(b.id)?.quarterTurns ?? 0, kind: b.kind, id: b.id.toString(), color: b.color, health: b.health, owned: b.owner.toHexString() === conn!.connectionId.toHexString(), x: b.x, y: b.y, z: b.z })) : [],
      units: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.armyUnit.iter())
        .filter(u => u.gameId === gameId)
        .map(u => ({ id: u.id.toString(), number: u.number, color: u.color, health: u.health, owned: u.owner.toHexString() === conn!.connectionId.toHexString() && !conn!.db.worker.unitId.find(u.id), worker: !!conn!.db.worker.unitId.find(u.id), x: u.x, y: u.y, z: u.z })) : [],
    });
  },
};

Object.assign(globalThis, { strongholdNetwork });

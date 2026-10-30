import { DbConnection, type ReducerEventContext } from './module_bindings';

type Action =
  | { action: 'connect'; uri: string; database: string }
  | { action: 'create'; title: string }
  | { action: 'join'; game_id: string }
  | { action: 'leave' }
  | {action:'set_gate';building_id:string;open:boolean}
  | {action:'dig_moat';start_x:number;start_z:number;end_x:number;end_z:number}
  | {action:'fill_moat';moat_id:string}
  | {action:'build_wall_stair';wall_id:string;base_x:number;base_z:number;top_x:number;top_z:number}
  | {action:'build_wall_kind';start_x:number;start_z:number;end_x:number;end_z:number;kind:string}
  | { action: 'assign_peasant'; building_id:string }
  | { action: 'unassign_peasant'; building_id:string }
  | { action:'assign_ox_cart'|'release_ox_cart';building_id:string }
  | { action:'hunt_animal';unit_ids:string[];animal_id:string }
  | { action: 'recruit_unit'; building_id: string; kind: string }
  | { action: 'build'; x: number; z: number; building: string; rotation?: number }
  | { action: 'build_wall'; start_x: number; start_z: number; end_x: number; end_z: number }
  | { action: 'remove_building'; building_id: string }
  | { action: 'move_to_roof'; unit_ids: string[]; building_id: string; x: number; z: number }
  | { action: 'attack_units'; unit_ids: string[]; target_unit_id: string }
  | { action: 'attack_building'; unit_ids: string[]; building_id: string }
  | { action: 'formation_targets'; unit_ids: string[]; start_x: number; start_z: number; end_x: number; end_z: number }
  | { action: 'targets'; unit_ids: string[]; x: number; z: number };

let conn: DbConnection | undefined;
let connected = false;
let ready = false;
let error: string | null = null;
let activeGame: bigint | undefined;
let unitSubscription: ReturnType<ReturnType<DbConnection['subscriptionBuilder']>['subscribe']> | undefined;
let reconnectTimer: ReturnType<typeof setTimeout> | undefined;
let reconnectDelay = 1000;

function cancelReconnect() {
  if (reconnectTimer !== undefined) clearTimeout(reconnectTimer);
  reconnectTimer = undefined;
}

function reconnectLater(uri: string, database: string, previous: DbConnection) {
  cancelReconnect();
  reconnectTimer = setTimeout(() => {
    reconnectTimer = undefined;
    if (conn === previous) connect(uri, database);
  }, reconnectDelay);
  reconnectDelay = Math.min(reconnectDelay * 2, 10000);
}

function connectionError(reason: unknown) {
  if (reason instanceof Error && reason.message) return reason.message;
  if (typeof reason === 'string' && reason) return reason;
  return 'Verbindung zum Spielserver unterbrochen. Verbinde erneut …';
}

function result(ctx: ReducerEventContext) {
  if (ctx.event.callerConnectionId?.toHexString() !== conn?.connectionId.toHexString()) return;
  const status = ctx.event.status;
  error = status.tag === 'Failed' ? status.value : status.tag === 'OutOfEnergy' ? 'Server is out of energy.' : null;
}

function connect(uri: string, database: string) {
  cancelReconnect();
  // SDK 1.1 otherwise downgrades https URLs to plain ws, which cloud hosts reject.
  // The sharing gateway supplies a same-origin endpoint; ordinary local builds
  // continue using the server configured by Rust.
  const sharedServer = typeof document === 'undefined' ? undefined
    : document.querySelector<HTMLMetaElement>('meta[name="stronghold-server"]')?.content;
  const socketUri = sharedServer ? new URL(sharedServer, window.location.href) : new URL(uri);
  if (socketUri.protocol === 'https:') socketUri.protocol = 'wss:';
  if (socketUri.protocol === 'http:') socketUri.protocol = 'ws:';
  // Ignore callbacks from a connection replaced by a reconnect.
  const previous = conn;
  conn = undefined;
  previous?.disconnect();
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
      error = null;
      reconnectDelay = 1000;
      db.subscriptionBuilder()
        .onApplied(() => { if (conn === db) ready = true; })
        .onError((ctx) => {
          if (conn !== db) return;
          ready = false;
          error = connectionError(ctx.event);
          reconnectLater(uri, database, db);
        })
        .subscribe(['SELECT * FROM game', 'SELECT * FROM participant']);
    })
    .onConnectError((_ctx, err) => {
      if (conn !== connection) return;
      connected = ready = false;
      error = connectionError(err);
      reconnectLater(uri, database, connection);
    })
    .onDisconnect((_ctx, err) => {
      if (conn !== connection) return;
      connected = ready = false;
      error = connectionError(err);
      reconnectLater(uri, database, connection);
    })
    .build();
  conn = connection;
  connection.reducers.onAssignPeasant(result);
  connection.reducers.onUnassignPeasant(result);
  connection.reducers.onAssignOxCart(result);
  connection.reducers.onReleaseOxCart(result);
  connection.reducers.onSetGate(result);connection.reducers.onDigMoat(result);connection.reducers.onFillMoat(result);connection.reducers.onBuildWallStair(result);connection.reducers.onBuildWallKind(result);
  connection.reducers.onHuntAnimal(result);
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
  connection.reducers.onAttackUnits(result);
  connection.reducers.onRecruitUnit(result);
}

export const strongholdNetwork = {
  disconnect() {
    cancelReconnect();
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
      case 'assign_peasant': conn.reducers.assignPeasant(BigInt(action.building_id)); break;
      case 'unassign_peasant': conn.reducers.unassignPeasant(BigInt(action.building_id)); break;
      case 'assign_ox_cart': conn.reducers.assignOxCart(BigInt(action.building_id)); break;
      case 'release_ox_cart': conn.reducers.releaseOxCart(BigInt(action.building_id)); break;
      case 'set_gate': conn?.reducers.setGate(BigInt(action.building_id),action.open);break;
      case 'dig_moat': conn?.reducers.digMoat(action.start_x,action.start_z,action.end_x,action.end_z);break;
      case 'fill_moat': conn?.reducers.fillMoat(BigInt(action.moat_id));break;
      case 'build_wall_stair': conn?.reducers.buildWallStair(BigInt(action.wall_id),action.base_x,action.base_z,action.top_x,action.top_z);break;
      case 'build_wall_kind': conn?.reducers.buildWallKind(action.start_x,action.start_z,action.end_x,action.end_z,action.kind);break;
      case 'hunt_animal': conn.reducers.huntAnimal(action.unit_ids.map(BigInt),BigInt(action.animal_id)); break;
      case 'leave': conn.reducers.leaveGame(); break;
      case 'recruit_unit': conn.reducers.recruitUnit(BigInt(action.building_id),action.kind); break;
      case 'build': conn.reducers.buildRotated(action.x, action.z, action.building, action.rotation ?? 0); break;
      case 'build_wall': conn.reducers.buildWall(action.start_x, action.start_z, action.end_x, action.end_z); break;
      case 'remove_building': conn.reducers.removeBuilding(BigInt(action.building_id)); break;
      case 'move_to_roof': conn.reducers.moveToRoof(action.unit_ids.map(BigInt), BigInt(action.building_id), action.x, action.z); break;
      case 'attack_units': conn.reducers.attackUnits(action.unit_ids.map(BigInt), BigInt(action.target_unit_id)); break;
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
        .subscribe([`SELECT * FROM army_unit WHERE game_id = ${gameId}`, `SELECT * FROM building WHERE game_id = ${gameId}`, `SELECT * FROM terrain_clearing WHERE game_id = ${gameId}`, `SELECT * FROM keep_population WHERE game_id = ${gameId}`, `SELECT * FROM settlement_needs WHERE game_id = ${gameId}`, `SELECT * FROM building_operation WHERE game_id = ${gameId}`, `SELECT * FROM resource_node WHERE game_id = ${gameId}`,`SELECT * FROM gate_state WHERE game_id = ${gameId}`,`SELECT * FROM moat WHERE game_id = ${gameId}`,`SELECT * FROM wall_stair WHERE game_id = ${gameId}`, `SELECT * FROM wildlife WHERE game_id = ${gameId}`, `SELECT * FROM ox_cart WHERE game_id = ${gameId}`, `SELECT * FROM fish_stock WHERE game_id = ${gameId}`, `SELECT * FROM building_rotation WHERE game_id = ${gameId}`, `SELECT * FROM tower_occupant WHERE game_id = ${gameId}`, `SELECT * FROM player_resources WHERE game_id = ${gameId}`, `SELECT * FROM warehouse_stock WHERE game_id = ${gameId}`, `SELECT * FROM granary_stock WHERE game_id = ${gameId}`, `SELECT * FROM worker WHERE game_id = ${gameId}`, `SELECT * FROM producer WHERE game_id = ${gameId}`, `SELECT * FROM unit_profile WHERE game_id = ${gameId}`, `SELECT * FROM arrow_projectile WHERE game_id = ${gameId}`]);
    }
    return JSON.stringify({
      connected, ready, error, game_id: gameId === undefined ? null : gameId.toString(),
      resources: ready && connected && conn && gameId !== undefined ? (() => {
        const stock=conn.db.playerResources.connectionId.find(conn.connectionId);
        const food=Array.from(conn.db.granaryStock.iter()).filter(s=>s.gameId===gameId&&s.owner.toHexString()===conn!.connectionId.toHexString())
          .reduce((n,s)=>({apples:n.apples+s.apples,bread:n.bread+s.bread,cheese:n.cheese+s.cheese,meat:n.meat+s.meat}),{apples:0,bread:0,cheese:0,meat:0});
        return stock?.gameId===gameId ? {wood:stock.wood,stone:stock.stone,iron:stock.iron,...food} : null;
      })() : null,
      games: ready && connected && conn ? Array.from(conn.db.game.iter())
        .sort((a, b) => a.id < b.id ? -1 : 1)
        .map(g => ({ id: g.id.toString(), title: g.title })) : [],
      keep_population: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.keepPopulation.iter())
        .filter(k=>k.gameId===gameId).map(k=>({building_id:k.buildingId.toString(),arrived:k.arrived,seconds:k.seconds})) : [],
      gates: ready&&connected&&conn&&gameId!==undefined?Array.from(conn.db.gateState.iter()).filter(n=>n.gameId===gameId).map(n=>({building_id:n.buildingId.toString(),open:n.open})):[],
      moats: ready&&connected&&conn&&gameId!==undefined?Array.from(conn.db.moat.iter()).filter(n=>n.gameId===gameId).map(n=>({id:n.id.toString(),owned:n.owner.toHexString()===conn!.connectionId.toHexString(),x:n.x,z:n.z})):[],
      stairs: ready&&connected&&conn&&gameId!==undefined?Array.from(conn.db.wallStair.iter()).filter(n=>n.gameId===gameId).map(n=>({id:n.id.toString(),wall_id:n.wallId.toString(),owned:n.owner.toHexString()===conn!.connectionId.toHexString(),base_x:n.baseX,base_y:n.baseY,base_z:n.baseZ,top_x:n.topX,top_y:n.topY,top_z:n.topZ})):[],
      resource_nodes: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.resourceNode.iter()).filter(n=>n.gameId===gameId).map(n=>({id:n.id.toString(),site:n.site,kind:n.kind,x:n.x,z:n.z,remaining:n.remaining,capacity:n.capacity,phase:n.phase,phase_ticks:n.phaseTicks})) : [],
      wildlife: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.wildlife.iter()).filter(n=>n.gameId===gameId).map(n=>({id:n.id.toString(),species:n.species,group_id:n.groupId,x:n.x,z:n.z,health:n.health,state:n.state,meat:n.meat})) : [],
      ox_carts: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.oxCart.iter()).filter(n=>n.gameId===gameId).map(n=>({building_id:n.buildingId.toString(),source_building:n.sourceBuilding?.toString()??null})) : [],
      fish_stocks: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.fishStock.iter()).filter(n=>n.gameId===gameId).map(n=>({building_id:n.buildingId.toString(),amount:n.amount})) : [],
      building_operations: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.buildingOperation.iter()).filter(n=>n.gameId===gameId).map(n=>({building_id:n.buildingId.toString(),closed:n.closed})) : [],
      settlement_needs: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.settlementNeeds.iter()).filter(n=>n.gameId===gameId).map(n=>({building_id:n.buildingId.toString(),happiness:n.happiness,food:n.food,variety:n.variety,housing:n.housing,water:n.water,health:n.health,leisure:n.leisure,goods:n.goods,employment:n.employment,immigration_seconds:n.immigrationSeconds})) : [],
      warehouses: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.warehouseStock.iter()).filter(s=>s.gameId===gameId).map(s=>({building_id:s.buildingId.toString(),wood:s.wood,stone:s.stone,iron:s.iron})) : [],
      granaries: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.granaryStock.iter()).filter(s=>s.gameId===gameId).map(s=>({building_id:s.buildingId.toString(),apples:s.apples,bread:s.bread,cheese:s.cheese,meat:s.meat})) : [],
      workers: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.worker.iter()).filter(w=>w.gameId===gameId).map(w=>({unit_id:w.unitId.toString(),source_building:w.sourceBuilding.toString(),resource:w.resource,cargo:w.cargo})) : [],
      producers: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.producer.iter()).filter(p=>p.gameId===gameId).map(p=>({building_id:p.buildingId.toString(),resource:p.resource,buffer:p.buffer})) : [],
      arrows: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.arrowProjectile.iter()).filter(a=>a.gameId===gameId).map(a=>({id:a.id.toString(),target_id:a.targetId.toString(),building:a.building,start_x:a.startX,start_y:a.startY,start_z:a.startZ,end_x:a.endX,end_y:a.endY,end_z:a.endZ,age:a.age,flight_seconds:a.flightSeconds})) : [],
      roof_occupants: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.towerOccupant.iter())
        .filter(o => o.gameId === gameId)
        .map(o => ({ unit_id: o.unitId.toString(), building_id: o.buildingId.toString(), slot: o.slot, next_building: o.nextBuilding?.toString() ?? null, next_slot: o.nextSlot })) : [],
      clearings: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.terrainClearing.iter())
        .filter(c => c.gameId === gameId).sort((a,b) => a.id < b.id ? -1 : a.id > b.id ? 1 : 0)
        .map(c => ({id:c.id.toString(),x:c.x,z:c.z,half_x:c.halfX,half_z:c.halfZ})) : [],
      buildings: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.building.iter())
        .filter(b => b.gameId === gameId)
        .map(b => ({ rotation: conn!.db.buildingRotation.buildingId.find(b.id)?.quarterTurns ?? 0, kind: b.kind, id: b.id.toString(), color: b.color, health: b.health, owned: b.owner.toHexString() === conn!.connectionId.toHexString(), x: b.x, y: b.y, z: b.z })) : [],
      units: ready && connected && conn && gameId !== undefined ? Array.from(conn.db.armyUnit.iter())
        .filter(u => u.gameId === gameId)
        .map(u => ({ id: u.id.toString(), kind: conn!.db.unitProfile.unitId.find(u.id)?.kind ?? '', activity: conn!.db.unitProfile.unitId.find(u.id)?.activity ?? 0, facing_x: conn!.db.unitProfile.unitId.find(u.id)?.facingX ?? 0, facing_z: conn!.db.unitProfile.unitId.find(u.id)?.facingZ ?? -1, number: u.number, color: u.color, health: u.health, owned: u.owner.toHexString() === conn!.connectionId.toHexString() && !conn!.db.worker.unitId.find(u.id), worker: !!conn!.db.worker.unitId.find(u.id), x: u.x, y: u.y, z: u.z })) : [],
    });
  },
};

Object.assign(globalThis, { strongholdNetwork });

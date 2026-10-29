// Exercise the real bridge and server through a disposable socket proxy.
import assert from 'node:assert/strict';
import http from 'node:http';
import type { Duplex } from 'node:stream';
import { strongholdNetwork } from './network';
const sockets = new Set<Duplex>();
let connections = 0;
const proxy = http.createServer();
proxy.on('upgrade', (request, socket, head) => {
  connections++;
  const upstream = http.request({host:'127.0.0.1',port:18766,path:request.url,
    headers:{...request.headers,host:'127.0.0.1:18766'}});
  upstream.on('upgrade',(response,peer,peerHead)=>{
    sockets.add(socket); sockets.add(peer);
    socket.write(`HTTP/1.1 ${response.statusCode} ${response.statusMessage}\r\n`);
    for(let i=0;i<response.rawHeaders.length;i+=2) socket.write(`${response.rawHeaders[i]}: ${response.rawHeaders[i+1]}\r\n`);
    socket.write('\r\n');
    if(peerHead.length) socket.write(peerHead);
    if(head.length) peer.write(head);
    socket.on('error',()=>peer.destroy()); peer.on('error',()=>socket.destroy());
    socket.on('close',()=>{sockets.delete(socket);peer.destroy()});
    peer.on('close',()=>{sockets.delete(peer);socket.destroy()});
    socket.pipe(peer).pipe(socket);
  });
  upstream.on('error',()=>socket.destroy()); upstream.end();
});
const delay=(ms:number)=>new Promise(resolve=>setTimeout(resolve,ms));
const snapshot=()=>JSON.parse(strongholdNetwork.snapshot());
async function until(test:()=>boolean,label:string){
  const deadline=Date.now()+20000;
  while(!test()){assert.ok(Date.now()<deadline,label);await delay(30)}
}
await new Promise<void>(resolve=>proxy.listen(0,'127.0.0.1',resolve));
const address=proxy.address(); assert.ok(address && typeof address!=='string');
try {
  const connect={action:'connect',uri:`http://127.0.0.1:${address.port}`,database:'stronghold-terrain-test'};
  strongholdNetwork.send(JSON.stringify(connect));
  await until(()=>snapshot().ready,'initial connection');
  for(const socket of sockets) socket.destroy();
  await until(()=>connections>=2 && snapshot().ready,'automatic reconnect after socket interruption');
  assert.equal(snapshot().error,null,'successful reconnect clears the error');
  strongholdNetwork.send(JSON.stringify({action:'create',title:`Reconnect test ${Date.now()}`}));
  await until(()=>!!snapshot().game_id,'create game works after reconnect');
  strongholdNetwork.disconnect();
  const count=connections;
  await delay(1500);
  assert.equal(connections,count,'intentional disconnect must not reconnect');
  assert.equal(snapshot().connected,false);
  console.log('PASS: automatic reconnect after real socket interruption, error cleared, game created, intentional disconnect stays disconnected.');
} finally {
  strongholdNetwork.disconnect();
  for(const socket of sockets) socket.destroy();
  proxy.close();
}

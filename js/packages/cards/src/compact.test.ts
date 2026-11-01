import { expect, it } from 'vitest';
import { Board, loadCatalog } from './index';
import { SHARED_CATALOG, generator } from './test-support';

it('uses each stack height in JS and in Rust output placement, including after load',async()=>{
  const catalog=await loadCatalog(SHARED_CATALOG);
  const random=generator(92);
  for(let trial=0;trial<24;trial++){
    const placement={kind:'free' as const,footprint:{width:1,height:1},stackOffset:.35};
    const board=await Board.create(catalog,trial,{outputPlacement:placement});
    const bush=board.spawn('berry_bush',Math.fround(random()*2),Math.fround(random()*2));
    const worker=board.spawn('worker',0,0);board.dropOn(worker,bush);
    for(let i=0;i<8;i++){
      const base=board.spawn('wood',Math.fround(random()*6-3),Math.fround(random()*6-3));
      for(let j=0;j<i%3;j++)board.dropOn(board.spawn('wood',0,0),base);
    }
    const source=board.cards().find(c=>c.id===bush)!;
    const near={x:Math.fround(source.x+1),y:source.y};
    const expected=board.freeSpot(near,placement.footprint,{stackOffset:placement.stackOffset});
    board.takeEvents();
    const output=board.tick(2).find(e=>e.event==='card_spawned');
    expect(output).toMatchObject({x:expected!.x,y:expected!.y});
    const restored=await Board.load(catalog,board.save(),{outputPlacement:placement});
    expect(restored.outputPlacement()).toEqual({...placement,stackOffset:Math.fround(.35)});
    restored.free();board.free();
  }
});

it('rejects negative or nonfinite stack offsets without changing placement',async()=>{
  const board=await Board.create(await loadCatalog(SHARED_CATALOG),1);
  for(const stackOffset of [-1,Infinity,NaN]) {
    expect(()=>board.setOutputPlacement({kind:'free',footprint:{width:1,height:1},stackOffset})).toThrow();
    expect(()=>board.freeSpot({x:0,y:0},{width:1,height:1},{stackOffset})).toThrow();
  }
  expect(board.outputPlacement()).toEqual({kind:'row'});board.free();
});

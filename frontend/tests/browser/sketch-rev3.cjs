// 0066 customer revision 3 smoke: the customer's arc refill (AC36) and point + line Coincident
// selection first (AC37, AC40), through the real pointer, key and rail events.
const h=require('./sketch-rev2-support.cjs');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const out='target/0066-browser/rev3';
(async()=>{
 fs.mkdirSync(out,{recursive:true});
 const browser=await h.launch();
 const {page,errors}=await h.fresh(browser);
 try{
 const notes=h.notes(page);
 const settle=()=>page.waitForTimeout(150);
 const shot=name=>page.screenshot({path:`${out}/${name}.png`});
 const px=188.6/50; // screen px per mm at 100 %
 const at=(x,y)=>[300+x*px,220+y*px];
 const click=async(x,y)=>{const [sx,sy]=at(x,y);await page.mouse.move(sx,sy);await settle();await page.mouse.click(sx,sy);await settle();};
 const mode=async name=>{
  for(let i=0;i<7;i++){
   const text=await page.locator('[data-sketch-mode-label]').textContent().catch(()=>'');
   if(text.startsWith(name))return;
   await h.alt(page);
  }
  throw new Error(`no mode ${name}`);
 };
 /** Moves 2 px off (x, y), waits for the capture marker, and clicks there. */
 const captured=async(x,y,what)=>{
  const [sx,sy]=at(x,y);
  await page.mouse.move(sx+2,sy-1);await settle();
  await notes.getByText('Endpoint',{exact:true}).waitFor();
  await shot(`capture-${what}`);
  await page.mouse.click(sx+2,sy-1);await settle();
 };
 // The chain: polyline arc, tangent line, tangent arc, tangent arc closed on the start.
 await h.tool(page,'Polyline');
 await click(20,20);
 const [mx,my]=at(70,60);await page.mouse.move(mx,my);await settle();
 await mode('Tangent arc');await click(40,40);
 await page.mouse.move(mx,my);await settle();
 await mode('Tangent line');await click(40,70);
 await page.mouse.move(mx,my);await settle();
 await mode('Tangent arc');await click(20,90);
 await captured(20,20,'close');
 await page.keyboard.press('Escape');await page.keyboard.press('Escape');
 await shot('01-closed-chain');
 // Delete segment 2, the tangent line.
 await h.tool(page,'Select');await settle();
 await click(40,55);
 await page.keyboard.press('Delete');await settle();
 await shot('02-segment-deleted');
 // Refill with Arc, centre-first: centre, then start and end on the two free endpoints.
 await h.tool(page,'Arc');
 const [cx,cy]=at(36,54);await page.mouse.move(cx,cy);await settle();
 await mode('Arc, centre');
 await click(36,54);
 await captured(40,40,'start');
 await captured(40,70,'end');
 await page.keyboard.press('Escape');await page.keyboard.press('Escape');
 await shot('03-refilled');
 const {graph}=await h.save(page,out,'refill');
 // A closed loop of four arcs: every arc end is shared by exactly two arcs, no duplicates.
 const arcs=Object.values(graph.entities).filter(e=>e.Arc).map(e=>e.Arc);
 assert.equal(arcs.length,4,'three chain arcs and the refill arc');
 const ends=[...new Set(arcs.flatMap(a=>[a.start,a.end]))];
 assert.equal(ends.length,4,'four shared endpoints');
 for(const id of ends)assert.equal(arcs.filter(a=>a.start===id||a.end===id).length,2,`${id} joins two arcs`);
 const pos=id=>graph.entities[id].Point.position;
 const points=Object.keys(graph.entities).filter(id=>graph.entities[id].Point);
 for(const id of ends){
  const close=points.filter(o=>o!==id&&Math.hypot(pos(o).x-pos(id).x,pos(o).y-pos(id).y)<0.01);
  assert.deepEqual(close,[],`no duplicate near ${id}`);
 }
 // Point + line Coincident, selection first: the point lands on the line's infinite line.
 await h.tool(page,'Polyline');
 await click(70,40);await click(100,50);
 await page.keyboard.press('Escape');await page.keyboard.press('Escape');
 await h.tool(page,'Point');await click(80,25);
 await page.keyboard.press('Escape');
 await h.tool(page,'Select');await settle();
 await click(80,25);
 await page.keyboard.down('Shift');await click(85,45);await page.keyboard.up('Shift');
 await h.rail(page,'coincident').click();await settle();
 await page.mouse.move(1200,850);await settle();
 await notes.getByRole('img',{name:'Coincident, point on line constraint',exact:true}).waitFor();
 await shot('04-point-on-line');
 const saved=await h.save(page,out,'point-on-line');
 const relations=Object.values(saved.graph.constraints).map(c=>Object.keys(c.relation)[0]);
 assert.ok(relations.includes('PointOn'),JSON.stringify(relations));
 assert.deepEqual(errors,[]);
 console.log('rev3 smoke passed');
 }catch(e){await page.screenshot({path:`${out}/failure.png`});throw e;}finally{await browser.close();}
})().catch(e=>{console.error(e);process.exit(1);});

// 0066 customer revision 4 smoke: shapes return to Select with their geometry (AC42), constraint
// tools return to Select (AC43), Horizontal and Vertical distance on two points selected by
// Shift-click and by marquee (AC44-AC48), through the real pointer, key and rail events.
const h=require('./sketch-rev2-support.cjs');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const out='target/0066-browser/rev4';
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
 const selectPressed=async what=>{
  const pressed=await page.getByRole('navigation',{name:'Sketch tools'}).getByRole('button',{name:'Select',exact:true}).getAttribute('aria-pressed');
  assert.equal(pressed,'true',`${what}: back in Select`);
 };
 const box=()=>page.getByRole('group',{name:'Dimension',exact:true});
 const type=async value=>{await box().waitFor();await page.keyboard.press('Control+a');await page.keyboard.type(value);await page.keyboard.press('Enter');await settle();};
 // AC44: the two new tools sit in the Constraints card after Dimension.
 const names=await page.getByRole('toolbar',{name:'Constraints'}).locator('[data-sketch-rail]').evaluateAll(b=>b.map(e=>e.dataset.sketchRail));
 assert.deepEqual(names.slice(-3),['dimension','horizontal-distance','vertical-distance']);
 assert.equal(await h.rail(page,'horizontal-distance').getAttribute('aria-label'),'Horizontal distance');
 assert.equal(await h.rail(page,'vertical-distance').getAttribute('aria-label'),'Vertical distance');
 // AC42: a rectangle, then a polyline ended by Enter, each back in Select.
 await h.tool(page,'Rectangle');await click(10,10);await click(40,30);
 await selectPressed('rectangle');
 await shot('01-rectangle-selected');
 await h.tool(page,'Polyline');await click(10,60);await click(40,60);await click(40,80);
 await page.keyboard.press('Enter');await settle();
 await selectPressed('polyline');
 await shot('02-chain-selected');
 // Two points, each back in Select.
 for(const [x,y] of [[70,20],[110,35]]){await h.tool(page,'Point');await click(x,y);await selectPressed('point');}
 await page.keyboard.press('Escape');await settle();
 // AC47/AC48: Shift-click two points, Horizontal distance applies at once, 40 mm typed.
 await click(70,20);
 await page.keyboard.down('Shift');await click(110,35);await page.keyboard.up('Shift');
 await h.rail(page,'horizontal-distance').click();await settle();
 await type('40');
 await notes.getByRole('button',{name:/^Horizontal distance ↔ 40 mm/}).waitFor();
 await selectPressed('horizontal distance');
 // The two points stay selected (AC43): Vertical distance applies at once too.
 await h.rail(page,'vertical-distance').click();await settle();
 await type('15');
 await notes.getByRole('button',{name:/^Vertical distance ↕ 15 mm/}).waitFor();
 await page.mouse.move(1200,850);await settle();
 await shot('03-axis-distances');
 // A marquee around the two points selects them; Dimension gives the aligned distance.
 await page.keyboard.press('Escape');await settle();
 const [x0,y0]=at(60,10),[x1,y1]=at(130,45);
 await page.mouse.move(Math.min(x0,x1),Math.min(y0,y1));await page.mouse.down();
 await page.mouse.move(Math.max(x0,x1),Math.max(y0,y1),{steps:8});await page.mouse.up();await settle();
 await h.rail(page,'dimension').click();await settle();
 await box().waitFor();
 assert.equal(await box().getByLabel('Sketch Distance').count(),1,'two points give the aligned distance');
 await page.keyboard.press('Escape');await settle();
 await selectPressed('dimension');
 await page.mouse.move(1200,850);await settle();
 await shot('04-three-distances');
 const texts=await notes.locator('[data-sketch-dimension-text]').allInnerTexts();
 console.log(JSON.stringify(texts));
 assert.ok(texts.includes('↔ 40 mm'),JSON.stringify(texts));
 assert.ok(texts.includes('↕ 15 mm'),JSON.stringify(texts));
 const {graph}=await h.save(page,out,'axis-distances');
 const relation=kind=>Object.values(graph.constraints).map(c=>c.relation[kind]).find(Boolean);
 const horizontal=relation('HorizontalDistance'),vertical=relation('VerticalDistance');
 assert.ok(horizontal&&vertical,JSON.stringify(graph.constraints));
 // Saved intent keeps the typed values; the solved geometry is asserted in Rust (AC45).
 assert.equal(horizontal.value.Literal.value,40,'40 mm along X');
 assert.equal(vertical.value.Literal.value,15,'15 mm along Y');
 assert.deepEqual(errors,[]);
 console.log('rev4 smoke passed');
 }catch(e){await page.screenshot({path:`${out}/failure.png`});throw e;}finally{await browser.close();}
})().catch(e=>{console.error(e);process.exit(1);});

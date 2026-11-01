// 0066 customer polish of revision 5 (2026-11-01): the one-field entry box has no label, shows at
// most three decimals and does not clip; a hovered, selected or armed-hovered axis is highlighted.
const h=require('./sketch-rev2-support.cjs');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const out='target/0066-rev5/polish';
(async()=>{
 fs.mkdirSync(out,{recursive:true});
 const browser=await h.launch();
 const {page,errors}=await h.fresh(browser);
 try{
 const settle=()=>page.waitForTimeout(150);
 const O=[640,609];
 const xy=([x,y])=>[O[0]+x,O[1]+y];
 const click=async(p,mods=[])=>{for(const m of mods)await page.keyboard.down(m);const [x,y]=xy(p);await page.mouse.move(x,y);await settle();await page.mouse.click(x,y);await settle();for(const m of mods)await page.keyboard.up(m);};
 const A=[-120,40],B=[-3,-60];
 await h.tool(page,'Polyline');
 await click(A);await click(B);
 await page.keyboard.press('Escape');await settle();await page.keyboard.press('Escape');await settle();
 await h.tool(page,'Select');await settle();
 await click(A);await click(B,['Shift']);
 await h.rail(page,'dimension').click();await settle();
 const box=page.locator('[data-sketch-entry]');
 const input=box.getByLabel('Sketch Distance');
 const value=await input.inputValue();
 console.log('entry value',value,'box text',JSON.stringify(await box.innerText()));
 assert.match(value,/^\d+(\.\d{1,3})?$/,'at most three decimals');
 assert.ok(!(await box.innerText()).includes('Distance'),'no label');
 const fits=await input.evaluate(el=>el.scrollWidth<=el.clientWidth);
 assert.ok(fits,'value not clipped');
 const b=await box.boundingBox();
 await page.screenshot({path:`${out}/entry-box.png`,clip:{x:b.x-80,y:b.y-80,width:b.width+160,height:b.height+160}});
 await page.keyboard.press('Escape');await settle();
 await page.keyboard.press('Escape');await settle();
 // Hover the Y axis away from geometry: the hover band.
 const Y=[0,150];
 await page.mouse.move(...xy(Y));await settle();await h.pump(page);
 await page.screenshot({path:`${out}/axis-hover.png`,clip:{x:O[0]-120,y:O[1]+60,width:240,height:180}});
 await click(Y);await page.mouse.move(...xy([150,200]));await settle();
 await page.screenshot({path:`${out}/axis-selected.png`,clip:{x:O[0]-120,y:O[1]+60,width:240,height:180}});
 // Armed Symmetric: hovering the X axis highlights it as the pick target.
 await page.keyboard.press('Escape');await settle();
 await h.rail(page,'symmetric').click();await settle();
 await page.mouse.move(...xy([150,0]));await settle();await h.pump(page);
 await page.screenshot({path:`${out}/axis-armed-hover.png`,clip:{x:O[0]+40,y:O[1]-90,width:240,height:180}});
 assert.deepEqual(errors,[]);
 console.log('polish shots passed');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exit(1);});

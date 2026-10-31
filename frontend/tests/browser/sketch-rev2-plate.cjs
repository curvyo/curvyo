// 0066 revision 2 UI review fixes on the customer's plate: 50 mm wide, closed by an R 25 mm
// semicircle, a 25 mm slot cut-out 12.5 mm from the left edge.
const h=require('./sketch-rev2-support.cjs');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const out='target/0066-browser/rev2-ux-fixed';
(async()=>{
 fs.mkdirSync(out,{recursive:true});
 const browser=await h.launch();
 const {page,errors}=await h.fresh(browser);
 try{
 const notes=h.notes(page);
 const settle=()=>page.waitForTimeout(150);
 const shot=name=>page.screenshot({path:`${out}/${name}.png`});
 const px=188.6/50; // screen px per mm at 100 %
 const L=400,T=300,R=L+50*px,B=T+50*px;
 // Outline: line (typed 50 mm, 0°), tangent arc, line back, close on the start.
 await h.tool(page,'Polyline');
 await page.mouse.click(L,T);await page.mouse.move(R-20,T+2);await settle();
 await page.keyboard.press('Tab');
 const line=page.getByRole('group',{name:'Line',exact:true});await line.waitFor();
 await page.keyboard.type('0');await page.keyboard.press('Enter');await settle();
 await line.locator('[role=alert]').waitFor();
 await shot('05-entry-refused');
 const marked=await line.locator('input[aria-invalid="true"]').count();
 assert.equal(marked,1,'only the refused field is marked');
 assert.equal(await page.getByRole('tabpanel').getByText('Must be more than 0 mm').count(),0,'no panel copy');
 await page.keyboard.press('Control+a');await page.keyboard.type('50');await page.keyboard.press('Tab');
 await page.keyboard.press('Control+a');await page.keyboard.type('0');await page.keyboard.press('Enter');await settle();
 for(let i=0;i<3;i++)await page.keyboard.press('Alt');
 await page.waitForFunction(()=>document.querySelector('[data-sketch-mode-label]')?.textContent.startsWith('Tangent arc'));
 await page.mouse.move(R+60,(T+B)/2);await settle();
 await page.mouse.move(R,B);await settle();await page.mouse.click(R,B);await settle();
 for(let i=0;i<3;i++)await page.keyboard.press('Alt');
 await page.waitForFunction(()=>document.querySelector('[data-sketch-mode-label]')?.textContent.startsWith('Line'));
 await page.keyboard.down('Control');await page.mouse.move(L+30,B+2);await settle();
 await notes.getByRole('img',{name:'Horizontal',exact:true}).waitFor();
 await shot('04-ctrl-lock');
 await page.mouse.move(L,B+1);await settle();await page.mouse.click(L,B+1);await page.keyboard.up('Control');await settle();
 await page.mouse.move(L,T+3);await settle();await page.mouse.click(L,T+3);await settle();
 await page.keyboard.press('Escape');await page.keyboard.press('Escape');
 await h.tool(page,'Select');
 // Constraints: vertical left edge, arc R 25 mm (radius by default). The top line's typed 50 mm
 // and 0° are already driving dimensions (0066 AC39): a Horizontal on it would be redundant.
 await page.mouse.click(L,(T+B)/2);await settle();await h.rail(page,'vertical').click();await settle();
 await notes.locator('[data-sketch-dimension-text]').filter({hasText:/^50 mm$/}).first().waitFor();
 // pick the arc at its rightmost point
 await page.mouse.click(R+25*px,(T+B)/2);await settle();
 // A preselected arc is dimensioned at once and its entry opens on it (0066 AC40).
 await h.rail(page,'dimension').click();await settle();
 const box=page.getByRole('group',{name:'Dimension',exact:true});await box.waitFor();
 assert.equal(await box.getByLabel('Sketch Radius').count(),1,'an arc dimensions its radius');
 await page.keyboard.press('Control+a');await page.keyboard.type('25');await page.keyboard.press('Enter');await settle();
 await page.keyboard.press('Escape');await page.keyboard.press('Escape');await h.tool(page,'Select');
 await notes.getByRole('button',{name:/^Radius R 25 mm/}).waitFor();
 // Slot cut-out: 12.5 mm from the left edge, 25 mm long.
 const sx=L+12.5*px,sy=(T+B)/2;
 await h.tool(page,'Slot');
 await page.mouse.click(sx,sy);await page.mouse.click(sx+25*px,sy);await page.mouse.click(sx+25*px,sy+4*px);await settle();
 await page.keyboard.press('Escape');await page.keyboard.press('Escape');
 await h.tool(page,'Select');await settle();
 // Spacing 25 mm (not the top line's typed 0°).
 const spacing=notes.locator('[data-sketch-dimension-text]').filter({hasNotText:/R |50 mm|°/});
 const sp=await spacing.first().boundingBox();
 await page.mouse.dblclick(sp.x+sp.width/2,sp.y+sp.height/2);
 await page.getByRole('group',{name:'Dimension',exact:true}).waitFor();
 await page.keyboard.press('Control+a');await page.keyboard.type('25');await page.keyboard.press('Enter');await settle();
 await page.keyboard.press('Escape');
 // 12.5 mm offset: the left edge plus the left cap centre (line plus point, change 2).
 await page.mouse.click(L,B-20);await settle();
 await page.keyboard.down('Shift');await page.mouse.click(sx,sy);await page.keyboard.up('Shift');await settle();
 await h.rail(page,'dimension').click();await settle();
 const off=page.getByRole('group',{name:'Dimension',exact:true});await off.waitFor();
 await shot('02-line-point-entry');
 assert.equal(await off.getByLabel('Sketch Distance').count(),1,'line plus point gives a distance');
 await page.keyboard.press('Control+a');await page.keyboard.type('12.5');await page.keyboard.press('Enter');await settle();
 await page.keyboard.press('Escape');await page.keyboard.press('Escape');await h.tool(page,'Select');
 await page.mouse.move(1200,850);await settle();
 const texts=await notes.locator('[data-sketch-dimension-text]').allInnerTexts();
 console.log(JSON.stringify(texts));
 await page.getByRole('tab',{name:'Sketches',exact:true}).click().catch(()=>{});
 await settle();
 await shot('plate');
 await page.screenshot({path:`${out}/plate-zoom.png`,clip:{x:300,y:180,width:520,height:400}});
 assert.ok(texts.includes('12.5 mm'),JSON.stringify(texts));
 assert.ok(texts.includes('50 mm'),JSON.stringify(texts));
 assert.ok(texts.some(t=>t.includes('R 25 mm')),JSON.stringify(texts));
 assert.deepEqual(errors,[]);
 console.log('plate passed');
 }catch(e){await page.screenshot({path:`${out}/plate-failure.png`});throw e;}finally{await browser.close();}
})().catch(e=>{console.error(e);process.exit(1);});

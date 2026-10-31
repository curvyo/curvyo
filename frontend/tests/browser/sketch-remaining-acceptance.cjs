// Independent real-event matrix on the revised interaction (0066 revisions 1 and 2): analytic
// variants cycled with Alt inside one rail tool, typed shapes in the on-canvas entry box, Polyline
// cancellation, typed Fillet refusal/correction with constraint-ID retention, Reference and an actual
// Save/Open round trip. Optional CURVYO_REMAINING_PHASE=fillet|reference selects one journey.
const assert=require('node:assert/strict');
const fs=require('node:fs');
const h=require('./sketch-rev2-support.cjs'),geometry=require('./sketch-geometry-oracles.cjs');
const out='target/0066-browser';
const save=(page,name)=>h.save(page,out,name);
const deferred=[]; // oracles that failed: reported after the rest of the journey ran, then the script fails
(async()=>{
 fs.mkdirSync(out,{recursive:true});
 const browser=await h.launch();
 try{
  for(const item of [
   {name:'circle-three',tool:'Circle',alt:1,label:'Circle, 3 points',picks:[[350,300],[350,300],[400,350],[450,300]],kind:'Circle'},
   {name:'ellipse-three',tool:'Ellipse',alt:1,label:'Ellipse, 3 points',picks:[[350,300],[450,300],[420,330]],kind:'Ellipse'},
   {name:'ellipse',tool:'Ellipse',label:'Ellipse, centre',picks:[[400,300],[450,300],[400,330]],kind:'Ellipse'},
   {name:'arc',tool:'Arc',label:'Arc, 3 points',picks:[[350,300],[400,350],[450,300]],kind:'Arc'},
   {name:'slot',tool:'Slot',picks:[[350,300],[450,300],[400,330]],kind:'Arc'},
   {name:'polyline',tool:'Polyline',picks:[[350,350],[350,300],[450,300]],kind:'Line',escape:true},
   {name:'typed-circle',tool:'Circle',picks:[[400,300]],typed:[['Sketch Radius','10 mm']],kind:'Circle'},
   {name:'typed-rectangle',tool:'Rectangle',picks:[[350,300]],typed:[['Sketch Width','30 mm'],['Sketch Height','20 mm']],kind:'Line'},
   {name:'typed-ellipse',tool:'Ellipse',picks:[[400,300]],typed:[['Sketch First axis','20 mm'],['Sketch Second axis','10 mm']],kind:'Ellipse'},
  ].filter(()=>!process.env.CURVYO_REMAINING_PHASE)){
   const {page,errors}=await h.fresh(browser);
   await h.tool(page,item.tool);
   assert.equal(await page.getByRole('combobox',{name:'Sketch tool variant'}).count(),0,'no variant menu');
   if(item.alt){ // The variant is chosen with Alt before the first pick; the label names it.
    await page.mouse.move(300,260);
    assert.equal(await h.alt(page,item.alt),item.label);
   } else if(item.label){await page.mouse.move(300,260);await h.pump(page);assert.equal(await page.locator('[data-sketch-mode-label]').textContent(),item.label);}
   for(const [x,y] of item.picks)await page.mouse.click(x,y);
   if(item.typed)await h.typeEntry(page,item.typed);
   if(item.escape)await page.keyboard.press('Escape');
   const saved=await save(page,`remaining-${item.name}`);
   geometry(item.name,saved.graph);
   const entities=Object.values(saved.graph.entities);
   assert(entities.some(e=>e[item.kind]),`${item.name} must create analytic ${item.kind}`);
   if(item.name==='polyline')assert.equal(entities.filter(e=>e.Line).length,2);
   if(item.name==='slot')assert.equal(entities.filter(e=>e.Arc).length,2);
   if(item.name==='typed-circle')assert(Math.abs(entities.find(e=>e.Circle).Circle.radius-10)<1e-6);
   assert.deepEqual(errors,[]);await page.close();
  }
  // Typed Fillet: a refused radius must not commit anything; the corrected radius commits once.
  if(!process.env.CURVYO_REMAINING_PHASE||process.env.CURVYO_REMAINING_PHASE==='fillet'){
   const {page,errors}=await h.fresh(browser);
   await h.tool(page,'Polyline');
   for(const [x,y]of [[350,400],[350,300],[450,300]])await page.mouse.click(x,y);
   await page.keyboard.press('Escape');
   const geometryList=await h.geometry(page);
   const lines=geometryList.getByRole('button',{name:/^line /});
   await lines.nth(0).click();await h.rail(page,'vertical').click();
   await lines.nth(1).click();await h.rail(page,'horizontal').click();
   await lines.nth(0).click();await lines.nth(1).click({modifiers:['Shift']});
   await h.tool(page,'Fillet');await page.mouse.move(360,310);
   await page.keyboard.press('Tab');
   const box=page.getByRole('group',{name:'Fillet',exact:true});await box.waitFor();
   const radius=box.getByRole('textbox',{name:'Sketch Radius',exact:true});
   await radius.fill('2 mm');await radius.fill('missing_radius');await page.keyboard.press('Enter');
   await page.getByRole('alert').first().waitFor();
   assert.equal(await radius.getAttribute('aria-invalid'),'true','the refused radius is marked invalid');
   // The refused radius commits nothing, not even the earlier 2 mm; the relation IDs are untouched.
   const ids=()=>page.locator('[data-sketch-dimension]').evaluateAll(n=>n.map(e=>e.getAttribute('data-sketch-dimension')));
   const arcs=()=>page.getByLabel('Sketch entities').getByRole('button',{name:/^arc /}).count(); // read only: clicking into Properties would close the entry box
   const refusedIds=await ids();assert.equal(refusedIds.length,2);assert.equal(await arcs(),0,'a refused radius commits nothing');
   await radius.fill('1 mm');await page.keyboard.press('Enter');
   await page.waitForFunction(()=>[...document.querySelectorAll('[aria-label="Sketch entities"] button')].some(b=>/^arc /.test(b.textContent)));
   assert.equal(await arcs(),1,'the corrected radius commits one arc');
   const committedIds=await ids();
   for(const id of refusedIds)assert(committedIds.includes(id),'fillet preserves original relation ID');
   const committed=await save(page,'remaining-fillet-committed');assert.equal(Object.values(committed.graph.entities).filter(e=>e.Arc).length,1);
   for(const id of refusedIds)assert(committed.graph.constraints[id],'saved fillet preserves original relation ID');
   assert.deepEqual(errors,[]);await page.close();
  }
  // Reference and real File/Open: the accepted dimension stays a read-only measurement after restoring bytes.
  if(!process.env.CURVYO_REMAINING_PHASE||process.env.CURVYO_REMAINING_PHASE==='reference'){
   const {page,errors}=await h.fresh(browser);
   await h.tool(page,'Circle');await page.mouse.click(400,300);await page.mouse.click(440,300);
   const status=await page.getByLabel('Sketch status').textContent();
   // Nothing selected: Dimension arms (a selection would apply at once, 0066 revision 3); pick the circle on its outline.
   await h.rail(page,'dimension').click();await page.mouse.move(440,300);await page.mouse.click(440,300);await page.mouse.move(560,380);await page.mouse.click(560,380);
   const entry=page.getByRole('group',{name:'Dimension',exact:true});await entry.waitFor();
   await entry.getByRole('checkbox',{name:'Reference',exact:true}).check();
   const measuredField=entry.getByRole('textbox',{name:'Sketch Diameter',exact:true}),shown=await measuredField.inputValue();
   await measuredField.focus();await page.keyboard.press('9');await entry.waitFor();assert.equal(await measuredField.inputValue(),shown,'typing into a reference field does not change the measurement');
   if(!(await measuredField.evaluate(e=>e.readOnly)))deferred.push('a reference field must be read-only (readonly attribute), as before the revision');
   await page.keyboard.press('Enter');
   await h.notes(page).locator('[data-sketch-dimension-text]').filter({hasText:/^\(⌀ /}).waitFor();
   assert.equal(await page.getByLabel('Sketch status').textContent(),status);
   const saved=await save(page,'remaining-reference');assert(Object.values(saved.graph.constraints).some(c=>c.reference));
   const chooser=page.waitForEvent('filechooser');await page.getByRole('button',{name:'Open…',exact:true}).click();await(await chooser).setFiles(saved.path);
   await page.getByRole('button',{name:'Workbench',exact:true}).waitFor();
   await h.menu(page,'Workbench','Sketch');
   await page.getByRole('tab',{name:'Sketches',exact:true}).click();
   const row=page.getByLabel('Sketch relationships').getByRole('button',{name:/Diameter.*Reference/});await row.focus();await row.press('Enter');
   assert.equal(await page.getByLabel('Diameter expression',{exact:true}).count(),0,'reference dimension must remain a read-only measured value');
   const measuredText=await page.getByLabel('Sketch relationships').locator('output').innerText();
   const measured=/^(-?\d+(?:\.\d+)?) mm$/.exec(measuredText);
   assert(measured,'reference measurement must expose a numeric millimetre output');
   // The radius is 40 screen pixels at 100% 96 dpi.
   assert(Math.abs(Number(measured[1])-80*25.4/96)<1e-6,'read-only output is the actual measured diameter, not a typed value');
   const reference=page.getByLabel('Sketch relationships').getByRole('checkbox',{name:'Reference',exact:true});
   assert(await reference.isChecked(),'reference mode remains explicit');
   assert(await reference.isEnabled(),'explicit switch to driving remains available');
   assert.deepEqual(errors,[]);await page.screenshot({path:`${out}/remaining-reference-reopened.png`});await page.close();
  }
  if(deferred.length)throw new Error('FAILED oracles: '+deferred.join('; '));
  console.log(process.env.CURVYO_REMAINING_PHASE ? `PASS targeted ${process.env.CURVYO_REMAINING_PHASE} browser journey` : 'PASS remaining creation variants (Alt) and typed shapes, duplicate correction, Polyline cancellation, typed Fillet refusal/correction, Reference and actual save-open keyboard editing');
 }catch(error){for(const context of browser.contexts())for(const page of context.pages()){console.log(await page.locator('body').innerText());await page.screenshot({path:`${out}/remaining-failure.png`});}throw error;}finally{await browser.close();}
})().catch(e=>{console.error(e);process.exit(1);});

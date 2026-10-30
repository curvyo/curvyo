// Prepared independent real-event matrix. Execute only with the exclusive browser slot.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {execFileSync}=require('node:child_process');
const fs=require('node:fs');
const geometry=require('./sketch-geometry-oracles.cjs');
const flatten=nodes=>nodes.flatMap(n=>n.objects?flatten(n.objects):[n]);
async function save(page,name){
 await page.getByRole('tab',{name:/^File/}).click();
 const pending=page.waitForEvent('download');await page.getByRole('button',{name:'Save',exact:true}).click();
 const path=`target/0066-browser/${name}.curvyo`;await(await pending).saveAs(path);
 const json=JSON.parse(execFileSync('python3',['-c','import zipfile,sys;print(zipfile.ZipFile(sys.argv[1]).read("document.json").decode())',path],{encoding:'utf8'}));
 return {path,json,graph:flatten(json.objects).find(n=>n.shape==='sketch')?.graph};
}
async function fresh(browser){
 const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[];
 page.on('pageerror',e=>errors.push(e.stack));
 await page.goto(process.env.CURVYO_BROWSER_URL||'http://127.0.0.1:5173/',{waitUntil:'networkidle'});
 await page.getByLabel('Workbench',{exact:true}).selectOption('sketch');
 await page.getByRole('button',{name:'New',exact:true}).click();
 return {page,errors};
}
async function sketches(page){await page.getByRole('tab',{name:'Sketches',exact:true}).click();}
(async()=>{
 fs.mkdirSync('target/0066-browser',{recursive:true});
 const browser=await chromium.launch({headless:true,args:['--no-sandbox','--enable-unsafe-webgpu','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
 try {
  for(const item of [
   {name:'circle-three',tool:'Circle',variant:'circle-three-points',picks:[[350,300],[350,300],[400,350],[450,300]],kind:'Circle'},
   {name:'ellipse-three',tool:'Ellipse',variant:'ellipse-three-points',picks:[[350,300],[450,300],[420,330]],kind:'Ellipse'},
   {name:'ellipse',tool:'Ellipse',picks:[[400,300],[450,300],[400,330]],kind:'Ellipse'},
   {name:'arc',tool:'Arc',picks:[[350,300],[400,350],[450,300]],kind:'Arc'},
   {name:'slot',tool:'Slot',picks:[[350,300],[450,300],[400,330]],kind:'Arc'},
   {name:'polyline',tool:'Polyline',picks:[[350,350],[350,300],[450,300]],kind:'Line',escape:true},
   {name:'typed-circle',tool:'Circle',picks:[[400,300]],typed:[['Sketch Radius','10 mm']],kind:'Circle'},
   {name:'typed-rectangle',tool:'Rectangle',picks:[[350,300]],typed:[['Sketch Width','30 mm'],['Sketch Height','20 mm']],kind:'Line'},
   {name:'typed-ellipse',tool:'Ellipse',picks:[[400,300]],typed:[['Sketch First axis','20 mm'],['Sketch Second axis','10 mm']],kind:'Ellipse'},
  ].filter(()=>!process.env.CURVYO_REMAINING_PHASE)){
   const {page,errors}=await fresh(browser);
   await page.getByRole('button',{name:item.tool,exact:true}).click();
   if(item.variant)await page.getByLabel('Sketch tool variant').selectOption(item.variant);
   for(const [x,y] of item.picks)await page.mouse.click(x,y);
   if(item.typed){for(const [label,value]of item.typed)await page.getByLabel(label,{exact:true}).fill(value);await page.getByLabel('Sketch tool options').getByRole('button',{name:'Apply',exact:true}).click();}
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
  // Typed Fillet: a refused current radius must not leave the previous preview committable.
  if(!process.env.CURVYO_REMAINING_PHASE||process.env.CURVYO_REMAINING_PHASE==='fillet'){
   const {page,errors}=await fresh(browser);
   await page.getByRole('button',{name:'Polyline',exact:true}).click();
   for(const [x,y]of [[350,400],[350,300],[450,300]])await page.mouse.click(x,y);
   await page.keyboard.press('Escape');await page.getByText(/^Geometry \(/).click();
   const lines=page.getByLabel('Sketch entities').getByRole('button',{name:/^line /});
   await lines.nth(0).click();await page.getByLabel('Constraint actions').getByRole('button',{name:'Vertical',exact:true}).click();
   await lines.nth(1).click();await page.getByLabel('Constraint actions').getByRole('button',{name:'Horizontal',exact:true}).click();
   await lines.nth(0).click();await lines.nth(1).click({modifiers:['Shift']});
   await page.getByRole('button',{name:'Fillet',exact:true}).click();
   const radius=page.getByLabel('Sketch Radius',{exact:true});await radius.fill('2 mm');
   await page.getByRole('button',{name:'Preview',exact:true}).click();
   await radius.fill('missing_radius');await page.getByRole('button',{name:'Preview',exact:true}).click();
   await page.getByLabel('Sketch tool options').getByRole('button',{name:'Apply',exact:true}).click();
   const refused=await save(page,'remaining-fillet-refused');assert.equal(Object.values(refused.graph.entities).filter(e=>e.Arc).length,0);
   await sketches(page);await radius.fill('1 mm');await page.getByRole('button',{name:'Preview',exact:true}).click();
   await page.getByLabel('Sketch tool options').getByRole('button',{name:'Apply',exact:true}).click();
   const committed=await save(page,'remaining-fillet-committed');assert.equal(Object.values(committed.graph.entities).filter(e=>e.Arc).length,1);
   assert.equal(Object.keys(refused.graph.constraints).length,2);
   for(const id of Object.keys(refused.graph.constraints))assert(committed.graph.constraints[id],'fillet preserves original relation ID');
   assert.deepEqual(errors,[]);await page.close();
  }
  // Reference and real File/Open: accepted dimension remains editable after restoring bytes.
  if(!process.env.CURVYO_REMAINING_PHASE||process.env.CURVYO_REMAINING_PHASE==='reference'){
   const {page,errors}=await fresh(browser);
   await page.getByRole('button',{name:'Circle',exact:true}).click();await page.mouse.click(400,300);await page.mouse.click(440,300);
   await page.getByText(/^Geometry \(/).click();await page.getByLabel('Sketch entities').getByRole('button',{name:/^circle /}).click();
   const status=await page.getByLabel('Sketch status').textContent();
   await page.getByLabel('Constraint actions').getByRole('button',{name:'Diameter',exact:true}).click();
   await page.getByLabel('New diameter').fill('99 mm');await page.getByLabel('Constraint actions').getByRole('checkbox',{name:'Reference',exact:true}).check();
   await page.getByLabel('Constraint actions').getByRole('button',{name:'Apply',exact:true}).click();
   assert.equal(await page.getByLabel('Sketch status').textContent(),status);
   const saved=await save(page,'remaining-reference');assert(Object.values(saved.graph.constraints).some(c=>c.reference));
   const chooser=page.waitForEvent('filechooser');await page.getByRole('button',{name:'Open…',exact:true}).click();await(await chooser).setFiles(saved.path);
   await page.waitForFunction(()=>document.querySelector('[aria-label="Workbench"]')?.value==='vector');
   await page.getByLabel('Workbench',{exact:true}).selectOption('sketch');
   await sketches(page);await page.waitForFunction(()=>[...document.querySelectorAll('[aria-label="Active sketch"] option')].some(o=>o.value));
   const value=await page.getByLabel('Active sketch').locator('option').evaluateAll(nodes=>nodes.find(n=>n.value)?.value);
   await page.getByLabel('Active sketch').selectOption(value);await page.getByLabel('Workbench',{exact:true}).selectOption('sketch');
   const row=page.getByLabel('Sketch relationships').getByRole('button',{name:/Diameter.*Reference/});await row.focus();await row.press('Enter');
   assert.equal(await page.getByLabel('Diameter expression',{exact:true}).count(),0,'reference dimension must remain a read-only measured value');
   const measuredText=await page.getByLabel('Sketch relationships').locator('output').innerText();
   const measured=/^(-?\d+(?:\.\d+)?) mm$/.exec(measuredText);
   assert(measured,'reference measurement must expose a numeric millimetre output');
   // Radius is40 screen pixels at100%96dpi; panel precision need not equal annotation rounding.
   assert(Math.abs(Number(measured[1])-80*25.4/96)<1e-6,'read-only output is actual measured diameter, not nominal99mm');
   const reference=page.getByLabel('Sketch relationships').getByRole('checkbox',{name:'Reference',exact:true});
   assert(await reference.isChecked(),'reference mode remains explicit');
   assert(await reference.isEnabled(),'explicit switch to driving remains available');
   assert.deepEqual(errors,[]);await page.screenshot({path:'target/0066-browser/remaining-reference-reopened.png'});await page.close();
  }
  console.log(process.env.CURVYO_REMAINING_PHASE ? `PASS targeted ${process.env.CURVYO_REMAINING_PHASE} browser journey` : 'PASS remaining creation variants/typed shapes, duplicate correction, Polyline cancellation, stale typed Fillet refusal/correction, Reference and actual save-open keyboard editing');
 }catch(error){for(const context of browser.contexts())for(const page of context.pages()){console.log(await page.locator('body').innerText());await page.screenshot({path:'target/0066-browser/remaining-failure.png'});}throw error;}finally{await browser.close();}
})();

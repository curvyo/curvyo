const {chromium}=require('playwright'),assert=require('node:assert/strict');
const {execFileSync}=require('node:child_process'),geometry=require('./sketch-geometry-oracles.cjs');
const flatten=ns=>ns.flatMap(n=>n.objects?flatten(n.objects):[n]);
const pump=page=>page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
(async()=>{
 const browser=await chromium.launch({headless:true,args:['--no-sandbox','--enable-unsafe-webgpu','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
 try{for(const stored of [false,true]){
  const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.goto(process.env.CURVYO_BROWSER_URL||'http://127.0.0.1:1421/',{waitUntil:'networkidle'});
  await page.getByRole('tab',{name:'Snapping',exact:true}).click();
  if(stored)await page.getByRole('switch',{name:'Snapping',exact:true}).click();
  for(const target of ['Objects','Guides','Nodes'])await page.getByRole('button',{name:target,exact:true}).click();
  const frame=await page.locator('[data-page-frames] > div').first().boundingBox();
  await page.getByLabel('Workbench',{exact:true}).selectOption('sketch');await page.getByRole('button',{name:'New',exact:true}).click();
  await page.getByRole('button',{name:'Rectangle',exact:true}).click();await page.mouse.click(350,300);
  await page.mouse.move(frame.x+frame.width/2+4,frame.y+frame.height/2+4);
  const field=page.getByLabel('Sketch Width',{exact:true});await field.fill('30 mm');await field.focus();await pump(page);
  const clip={x:300,y:180,width:650,height:600};
  const before=await page.screenshot({clip});
  await field.press('Alt');await pump(page); // A full tap must restore the same pending geometry.
  assert.equal(await field.inputValue(),'30 mm');assert(await field.evaluate(e=>e===document.activeElement));
  await page.keyboard.down('Alt');await pump(page);
  const held=await page.screenshot({clip});assert(!before.equals(held),'Alt key-only while numeric field focused changes preview at the unchanged pointer');
  await page.keyboard.up('Alt');await pump(page);
  const restored=await page.screenshot({clip});assert(before.equals(restored),'Alt release restores preview without moving pinned anchor');
  assert.equal(await field.inputValue(),'30 mm');
  await page.getByLabel('Sketch Height',{exact:true}).fill('20 mm');
  await page.getByLabel('Sketch tool options').getByRole('button',{name:'Apply',exact:true}).click();
  await page.getByRole('tab',{name:/^File/}).click();const download=page.waitForEvent('download');await page.getByRole('button',{name:'Save',exact:true}).click();
  const path=`target/0066-browser/focused-alt-${stored}.curvyo`;await(await download).saveAs(path);
  const data=JSON.parse(execFileSync('python3',['-c','import zipfile,sys;print(zipfile.ZipFile(sys.argv[1]).read("document.json").decode())',path],{encoding:'utf8'}));
  geometry('typed-rectangle',flatten(data.objects).find(n=>n.shape==='sketch').graph);
  assert.equal(data.snapping.enabled,stored);
  assert.deepEqual(errors,[]);await page.close();
 }
 console.log('PASS focused numeric-field key-only Alt on/off restores actual canvas preview, preserves draft/focus and commits typed dimensions; both stored states');
 }finally{await browser.close();}
})();

const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {execFileSync}=require('node:child_process');
const path=require('node:path');
const artifact=name=>path.resolve('target/browser-check',name);
const pump=page=>page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
const near=(actual,expected)=>assert(Math.abs(actual-expected)<1e-8,`${actual} != ${expected}`);
const shapes=nodes=>nodes.flatMap(node=>node.shape==='layer'||node.shape==='group'?shapes(node.objects||[]):[node]);
async function exported(page,name){
  await page.getByRole('tab',{name:/^File/}).click();
  const download=page.waitForEvent('download');
  await page.getByRole('button',{name:'Save',exact:true}).click();
  const filename=artifact(`${name}.curvyo`);
  await(await download).saveAs(filename);
  return JSON.parse(execFileSync('python3',['-c','import zipfile,sys; print(zipfile.ZipFile(sys.argv[1]).read("document.json").decode())',filename],{encoding:'utf8'}));
}
async function setup(browser){
  const page=await browser.newPage({viewport:{width:1440,height:1000}});
  const errors=[]; page.on('pageerror',error=>errors.push(error.message));
  await page.goto(process.env.CURVYO_BROWSER_URL||'http://127.0.0.1:5173/',{waitUntil:'networkidle'});
  await page.getByRole('tab',{name:'Snapping',exact:true}).click();
  const enabled=page.getByRole('switch',{name:'Snapping',exact:true});
  await enabled.click();
  for(const name of ['Objects','Guides','Nodes']) await page.getByRole('button',{name,exact:true}).click();
  const frame=await page.locator('[data-page-frames] > div').first().boundingBox();
  return {page,errors,enabled,at:(x,y)=>({x:frame.x+frame.width*x/210,y:frame.y+frame.height*y/297}),frame};
}
async function clickRectangle(state,name){
  const {page,frame}=state;
  await page.getByRole('button',{name:/^Rectangle tool/}).click();
  const start={x:frame.x+frame.width/4+4,y:frame.y+frame.height/4+4};
  const end={x:frame.x+frame.width/2+3,y:frame.y+frame.height/2+3};
  await page.mouse.move(start.x,start.y);await pump(page);
  await page.screenshot({path:artifact(`${name}-hover.png`)});
  await page.mouse.click(start.x,start.y);await pump(page);
  await page.mouse.move(end.x,end.y);await pump(page);
  if(name==='axes-both') await page.mouse.click(end.x,end.y);else await page.keyboard.press('Enter');await pump(page);
  assert.equal(await page.getByRole('textbox',{name:'Width',exact:true}).count(),0,'placement must commit');
}
(async()=>{
  const browser=await chromium.launch({headless:true,args:['--no-sandbox','--enable-unsafe-webgpu','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
  for(const axis of ['both','x','y','none']){
    const state=await setup(browser),{page,errors}=state;
    const x=page.getByRole('button',{name:'Horizontal (X)',exact:true}),y=page.getByRole('button',{name:'Vertical (Y)',exact:true});
    assert.equal(await x.getAttribute('aria-pressed'),'true');assert.equal(await y.getAttribute('aria-pressed'),'true');
    if(axis==='y'||axis==='none') await x.click();
    if(axis==='x'||axis==='none') await y.click();
    if(axis==='y'){
      await x.focus();await page.keyboard.press('Control+z');assert.equal(await x.getAttribute('aria-pressed'),'true');
      await page.keyboard.press('Control+Shift+z');assert.equal(await x.getAttribute('aria-pressed'),'false');
    }
    await clickRectangle(state,`axes-${axis}`);
    const data=await exported(page,`axes-${axis}`),objects=shapes(data.objects);
    assert.equal(objects.length,1);assert.equal(objects[0].shape,'rect');
    const bounds=objects[0].bounds;
    if(axis==='both'||axis==='x'){near(bounds.origin.x,52.5);near(bounds.width,52.5);}else assert(bounds.origin.x>53.2&&bounds.origin.x<53.9);
    if(axis==='both'||axis==='y'){near(bounds.origin.y,74.25);near(bounds.height,74.25);}else assert(bounds.origin.y>74.9&&bounds.origin.y<75.7);
    assert.equal(data.snapping.x,axis==='both'||axis==='x');assert.equal(data.snapping.y,axis==='both'||axis==='y');
    assert.deepEqual(errors,[]);await page.close();
  }
  for(const [stored,ctrl] of [[true,false],[false,false],[false,true]]){
    const state=await setup(browser),{page,errors,enabled,at}=state;
    await clickRectangle(state,`alt-${stored}-${ctrl}`);
    if(!stored) await enabled.click();
    await page.getByRole('button',{name:/^Select tool/}).click();
    const start=at(78.75,111.375),end=at(94.75,111.375);
    await page.mouse.move(start.x,start.y);await page.mouse.down();await page.mouse.move(end.x,end.y);await pump(page);
    if(ctrl) await page.keyboard.down('Control');
    await page.keyboard.down('Alt');await pump(page);
    assert.equal(await page.getByRole('tab',{name:'Snapping',exact:true}).getAttribute('aria-description'),stored?'On':'Off');
    assert.equal(await page.locator('[data-move-badge="copy"]').count(),0,'Alt move has no plus badge');
    const readout=await page.getByText(/^Δ/).allTextContents();
    assert(readout.length>0,'live move readout is present');
    assert(readout.every(text=>!text.includes('Copy')),'Alt move readout does not claim Copy');
    await page.screenshot({path:artifact(`alt-${stored}-${ctrl}-held.png`)});
    await page.mouse.up();await page.keyboard.up('Alt');if(ctrl) await page.keyboard.up('Control');await pump(page);
    const data=await exported(page,`alt-${stored}-${ctrl}`),objects=shapes(data.objects);
    assert.equal(objects.length,1,'Ctrl+Alt must not copy');
    if(stored) assert(Math.abs(objects[0].bounds.origin.x-68.5)<0.01);else near(objects[0].bounds.origin.x,70);
    near(objects[0].bounds.origin.y,74.25);assert.equal(data.snapping.enabled,stored);
    assert.deepEqual(errors,[]);await page.close();
  }
  console.log('PASS feedback browser: all four axis combinations persisted with exact permitted-axis click geometry; axes undo/redo; no-button hover and click/Enter placement confirmation; Alt on→off/off→on during real moves incl Ctrl without copying or changing stored state; no JS errors');
  await browser.close();
})().catch(error=>{console.error(error);process.exit(1)});

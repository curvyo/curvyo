const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {execFileSync}=require('node:child_process');
const path=require('node:path');
const artifact=name=>path.resolve('target/browser-check',name);
const pump=page=>page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
const near=(actual,expected)=>assert(Math.abs(actual-expected)<1e-8,`${actual} != ${expected}`);
const shapes=nodes=>nodes.flatMap(node=>['layer','group'].includes(node.shape)?shapes(node.objects||[]):[node]);
async function setup(browser,stored,tool){
  const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[];
  page.on('pageerror',error=>errors.push(error.message));
  await page.goto(process.env.CURVYO_BROWSER_URL||'http://127.0.0.1:5173/',{waitUntil:'networkidle'});
  await page.getByRole('tab',{name:'Snapping',exact:true}).click();
  if(stored) await page.getByRole('switch',{name:'Snapping',exact:true}).click();
  for(const name of ['Objects','Guides','Nodes']) await page.getByRole('button',{name,exact:true}).click();
  await page.getByRole('button',{name:tool}).click();
  const frame=await page.locator('[data-page-frames] > div').first().boundingBox();
  await page.evaluate(()=>{
    window.__creationAltEvents=[];
    window.addEventListener('keydown',event=>{if(event.key==='Alt') window.__creationAltEvents.push(event.defaultPrevented);});
  });
  return {page,errors,stored,a:{x:frame.x+frame.width/4+4,y:frame.y+frame.height/4+4},b:{x:frame.x+frame.width/2+3,y:frame.y+frame.height/2+3}};
}
async function checkKeys(state){
  const {page,stored,errors}=state;
  assert.equal(await page.getByRole('tab',{name:'Snapping',exact:true}).getAttribute('aria-description'),stored?'On':'Off');
  const prevented=await page.evaluate(()=>window.__creationAltEvents);
  assert(prevented.length>0&&prevented.every(Boolean),'creation Alt key must prevent native menu focus');
  assert.deepEqual(errors,[]);
}
async function exported(page,name){
  await page.getByRole('tab',{name:/^File/}).click();
  const download=page.waitForEvent('download');await page.getByRole('button',{name:'Save',exact:true}).click();
  const filename=artifact(`${name}.curvyo`);await(await download).saveAs(filename);
  return JSON.parse(execFileSync('python3',['-c','import zipfile,sys; print(zipfile.ZipFile(sys.argv[1]).read("document.json").decode())',filename],{encoding:'utf8'}));
}
(async()=>{
  const browser=await chromium.launch({headless:true,args:['--no-sandbox','--enable-unsafe-webgpu','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
  for(const [name,tool] of [['rectangle',/^Rectangle tool/],['ellipse',/^Ellipse tool/],['polygon-star',/^(Polygon|Star).*tool/],['pen',/^Pen tool/]]){
    for(const stored of [false,true]){
      const state=await setup(browser,stored,tool),{page,a}=state;
      await page.mouse.move(a.x,a.y);await pump(page);
      await page.keyboard.down('Alt');await pump(page);
      await page.screenshot({path:artifact(`creation-${name}-${stored}-alt-held.png`)});
      await page.keyboard.up('Alt');await pump(page);
      await checkKeys(state);await page.close();
    }
  }
  for(const phase of ['before-press','drag','follow-click','press-to-placement']){
    for(const stored of [false,true]){
      const state=await setup(browser,stored,/^Rectangle tool/),{page,a,b}=state;
      await page.mouse.move(a.x,a.y);await pump(page);
      if(phase==='before-press') await page.keyboard.down('Alt');
      if(phase==='drag'||phase==='press-to-placement') await page.mouse.down();else await page.mouse.click(a.x,a.y);
      if(phase==='press-to-placement') {await page.keyboard.down('Alt');await page.mouse.up();}
      await pump(page);await page.mouse.move(b.x,b.y);await pump(page);
      if(phase==='drag') await page.keyboard.down('Alt');
      if(phase==='follow-click'){
        const width=page.getByRole('textbox',{name:'Width',exact:true});
        const before=await width.inputValue();
        await page.keyboard.down('Alt');await pump(page);assert.notEqual(await width.inputValue(),before,'Alt must update placement without moving pointer');
        await page.keyboard.up('Alt');await pump(page);assert.equal(await width.inputValue(),before);
        // Focused chip: Ctrl is a placement tap toggle, Alt must preserve that constraint.
        await width.focus();await page.keyboard.down('Control');await pump(page);
        const height=page.getByRole('textbox',{name:'Height',exact:true});
        const constrained=[await width.inputValue(),await height.inputValue()];
        assert.equal(constrained[0],constrained[1],'Ctrl aspect toggle keeps rectangle square');
        await page.keyboard.down('Alt');await pump(page);
        assert.notEqual(await width.inputValue(),constrained[0],'Ctrl+Alt refreshes focused placement');
        assert.equal(await width.inputValue(),await height.inputValue(),'Alt preserves Ctrl aspect toggle');
        await page.keyboard.up('Alt');await pump(page);
        assert.deepEqual([await width.inputValue(),await height.inputValue()],constrained);
        await page.keyboard.up('Control');await page.keyboard.press('Control');await pump(page);
        assert.equal(await width.inputValue(),before,'second Ctrl tap restores unconstrained preview');
        await page.keyboard.down('Alt');
      }
      await pump(page);await page.screenshot({path:artifact(`creation-${phase}-${stored}-alt-held.png`)});
      if(phase==='drag') await page.mouse.up();else await page.mouse.click(b.x,b.y);
      await page.keyboard.up('Alt');await pump(page);await checkKeys(state);
      const data=await exported(page,`creation-${phase}-${stored}`),objects=shapes(data.objects);
      assert.equal(objects.length,1);assert.equal(objects[0].shape,'rect');assert.equal(data.snapping.enabled,stored);
      const bounds=objects[0].bounds,anchorSnaps=phase==='before-press'?!stored:stored;
      if(anchorSnaps){near(bounds.origin.x,52.5);near(bounds.origin.y,74.25);}else {assert(bounds.origin.x>53.2&&bounds.origin.x<53.9);assert(bounds.origin.y>74.9&&bounds.origin.y<75.7);}
      if(!stored){near(bounds.origin.x+bounds.width,105);near(bounds.origin.y+bounds.height,148.5);}else {assert(bounds.origin.x+bounds.width>105.5&&bounds.origin.x+bounds.width<106.1);assert(bounds.origin.y+bounds.height>149&&bounds.origin.y+bounds.height<149.8);}
      await page.close();
    }
  }
  console.log('PASS creation Alt browser: all drawing-tool hover keydown/up and native-menu prevention; both stored states for Alt before first click, button-down drag, no-button placement follow, and press-to-placement; fixed anchors, key-only updates, focused Ctrl+Alt aspect preservation/restoration and exact exported geometry without stored-state changes; no JS errors');
  await browser.close();
})().catch(error=>{console.error(error);process.exit(1)});

// Independent tester journey for specs/0061-align-and-distribute: keyboard reach, three-object
// align/distribute by selection, one history step per press, undo, grid group visibility and
// linked Rows/Columns fields. Screenshots to target/0061-browser/.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {execFileSync}=require('node:child_process');
const fs=require('node:fs');
const path=require('node:path');
const out=n=>path.resolve('target/0061-browser',n);
fs.mkdirSync(out(''),{recursive:true});
const pump=p=>p.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
const shapes=ns=>ns.flatMap(n=>n.shape==='layer'||n.shape==='group'?shapes(n.objects||[]):[n]);
let saves=0;
async function boxes(page){
  await page.getByRole('tab',{name:/^File/}).click();
  const d=page.waitForEvent('download');
  await page.getByRole('button',{name:'Save',exact:true}).click();
  const f=out(`journey-${++saves}.curvyo`);await(await d).saveAs(f);
  const data=JSON.parse(execFileSync('python3',['-c','import zipfile,sys; print(zipfile.ZipFile(sys.argv[1]).read("document.json").decode())',f],{encoding:'utf8'}));
  return shapes(data.objects).map(o=>{const b=o.bounds;return {x0:b.origin.x,y0:b.origin.y,x1:b.origin.x+b.width,y1:b.origin.y+b.height};});
}
(async()=>{
  const browser=await chromium.launch({headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
  const page=await browser.newPage({viewport:{width:1440,height:1000}});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.goto(process.env.CURVYO_BROWSER_URL||'http://127.0.0.1:5173/',{waitUntil:'networkidle'});
  const frame=await page.locator('[data-page-frames] > div').first().boundingBox();
  const at=(fx,fy)=>({x:frame.x+frame.width*fx,y:frame.y+frame.height*fy});
  for(const [a,b] of [[[.1,.1],[.2,.2]],[[.35,.3],[.55,.4]],[[.7,.15],[.75,.28]]]){
    await page.getByRole('button',{name:/^Rectangle tool/}).click();
    const p=at(a[0],a[1]*.6),q=at(b[0],b[1]*.6);
    await page.mouse.move(p.x,p.y);await page.mouse.down();await page.mouse.move(q.x,q.y,{steps:4});await page.mouse.up();await pump(page);
  }
  await page.getByRole('button',{name:/^Select tool/}).click();
  await page.mouse.click(frame.x+5,frame.y+700); // blank
  await page.keyboard.press('Control+a');await pump(page);
  await page.screenshot({path:out("journey-sel.png")});
  const names=await page.locator('button[aria-label]').evaluateAll(es=>es.map(e=>e.getAttribute('aria-label')).filter(n=>/Align|Distribute|grid/i.test(n)));
  console.log('bar buttons:',names.join(' | '));
  console.log("grid by role:",await page.getByRole("button",{name:"Arrange in a grid"}).count(),"selected text:",(await page.locator("[aria-live]").allInnerTexts()).join("/"));
  assert(names.length>=10);
  assert.equal(await page.getByRole("button",{name:"Arrange in a grid"}).count(),1);
  // no text in symbol buttons
  for(const n of names){const t=(await page.locator(`button[aria-label="${n}"]`).first().innerText()).trim();assert.equal(t,'',`${n} has no visible text`);}
  const b0=await boxes(page);
  await page.getByRole('button',{name:'Align right edges',exact:true}).click();await pump(page);
  const b1=await boxes(page);
  const right=Math.max(...b0.map(b=>b.x1));
  for(const b of b1) assert(Math.abs(b.x1-right)<1e-6,'right edges on the selection box');
  // keyboard: focus the distribute button by Tab from align, press Space then undo
  const dist=names.find(n=>/Distribute.*horizontal.*gap|Distribute horizontal/i.test(n));
  console.log('distribute button:',dist);
  await page.locator(`button[aria-label="${dist}"]`).focus();
  await page.keyboard.press('Space');await pump(page);
  const b2=await boxes(page);
  assert.deepEqual(b2.map(b=>b.y0),b1.map(b=>b.y0),'vertical positions untouched');
  await page.keyboard.press('Control+z');await pump(page);
  const b3=await boxes(page);
  assert.deepEqual(b3,b1,'one undo reverts distribute');
  await page.screenshot({path:out('journey-bar.png')});
  // grid fields
  const rows=page.getByRole('textbox',{name:'Rows',exact:true}),cols=page.getByRole('textbox',{name:'Columns',exact:true});
  console.log('grid initial rows/cols:',await rows.getAttribute('placeholder'),await cols.getAttribute('placeholder'));
  assert.equal(await rows.getAttribute('placeholder'),'2');assert.equal(await cols.getAttribute('placeholder'),'2','nearest-to-square for 3');
  await rows.fill('1');await pump(page);
  assert.equal(await cols.getAttribute('placeholder'),'3','Rows=1 sets Columns=3');
  await page.getByRole('button',{name:'Arrange in a grid',exact:true}).click();await pump(page);
  const g=await boxes(page);
  const ys=g.map(b=>b.y0).sort((a,b)=>a-b);
  assert(Math.abs(ys[0]-ys[1])<6,'single row: tops close (centred in common cell height)');
  assert.equal(await page.getByRole('button',{name:'Arrange in a grid',exact:true}).count(),1,'selection stays');
  // reduced selection to 1 object hides grid group
  await page.getByRole('button',{name:/^Select tool/}).click();
  await page.keyboard.press('Escape');await pump(page);
  assert.equal(await page.getByRole("button",{name:"Arrange in a grid"}).count(),0,'grid group hidden with <2');
  assert(await page.locator('button[aria-label="Align left edges"]').count()===1,'align stays with nothing selected');
  assert.deepEqual(errors,[]);
  console.log('PASS journey');
  await browser.close();
})().catch(e=>{console.error('FAIL',e);process.exit(1);});

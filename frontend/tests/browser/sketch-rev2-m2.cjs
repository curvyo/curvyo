// 0066 revision 2 M2 smoke: mode label and Alt, on-canvas entry box, Ctrl lock, endpoint capture.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const out='target/0066-browser/rev2-m2';
(async()=>{
 fs.mkdirSync(out,{recursive:true});
 const browser=await chromium.launch({headless:true,args:['--no-sandbox','--enable-unsafe-webgpu','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
 try {
  const page=await browser.newPage({viewport:{width:1440,height:1000}}), errors=[];
  page.on('pageerror',e=>errors.push(e.stack));
  page.setDefaultTimeout(10000);
  await page.goto(process.env.CURVYO_BROWSER_URL||'http://127.0.0.1:1421/',{waitUntil:'networkidle'});
  await page.getByRole('button',{name:'Workbench',exact:true}).click();
  await page.getByRole('menuitemradio',{name:'Sketch',exact:true}).click();
  assert.equal(await page.getByRole('form',{name:'Sketch tool options'}).count(),0,'no options bar');
  await page.getByRole('navigation',{name:'Sketch tools'}).getByRole('button',{name:'Polyline',exact:true}).click();
  const label=page.locator('[data-sketch-mode-label]');
  const notes=page.getByLabel('Sketch annotations');
  await page.mouse.move(400,300);
  await page.waitForFunction(()=>document.querySelector('[data-sketch-mode-label]')?.textContent==='Line');
  await page.mouse.click(400,300);
  await page.mouse.move(520,300);
  // Tab opens the entry box at the segment, Escape closes it and keeps the pick.
  await page.keyboard.press('Tab');
  const box=page.getByRole('group',{name:'Line',exact:true});
  await box.waitFor();
  assert.equal(await page.evaluate(()=>document.activeElement?.getAttribute('aria-label')),'Sketch Length');
  await page.screenshot({path:`${out}/entry-box.png`});
  await page.keyboard.press('Escape');
  assert.equal(await box.count(),0);
  await page.mouse.click(520,300);
  // Alt cycles every mode once per press; the label follows.
  const seen=[];
  for(let i=0;i<6;i++){await page.keyboard.press('Alt');seen.push(await label.textContent());}
  assert.deepEqual(seen,['Perpendicular line','Tangent line','Tangent arc','Arc left','Arc right','Line']);
  await page.mouse.move(600,250);await page.keyboard.press('Alt');await page.keyboard.press('Alt');await page.keyboard.press('Alt');
  await page.waitForFunction(()=>document.querySelector('[data-sketch-mode-label]')?.textContent==='Tangent arc');
  await page.screenshot({path:`${out}/alt-label-tangent-arc.png`});
  for(let i=0;i<3;i++)await page.keyboard.press('Alt');
  assert.equal(await label.textContent(),'Line');
  // Ctrl locks the direction; at 0 degrees the pending Horizontal badge shows before the commit.
  await page.keyboard.down('Control');
  await page.mouse.move(640,306);
  await notes.getByRole('img',{name:'Horizontal',exact:true}).waitFor();
  assert.equal(await page.locator('[data-move-badge="lock-angle"]').count(),1,'angle lock badge');
  await notes.getByText(/, 0°$/).waitFor();
  await page.screenshot({path:`${out}/ctrl-lock-horizontal.png`});
  await page.mouse.click(640,306);
  await page.keyboard.up('Control');
  // Capture pulls onto the chain start within 12 px and closes the chain on one point.
  await page.mouse.move(640,420);await page.mouse.click(640,420);
  await page.mouse.move(408,306);
  await notes.getByText('Endpoint',{exact:true}).waitFor();
  await page.screenshot({path:`${out}/endpoint-capture.png`});
  await page.mouse.click(408,306);
  await page.getByRole('tab',{name:'Sketches',exact:true}).click();
  const relations=await page.getByLabel('Sketch relationships').innerText();
  assert.match(relations,/Horizontal/);
  await page.screenshot({path:`${out}/closed-chain.png`});
  assert.deepEqual(errors,[]);
  console.log('rev2-m2 smoke passed');
 } finally { await browser.close(); }
})().catch(e=>{console.error(e);process.exit(1);});

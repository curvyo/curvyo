// 0066 revision 2 M3 smoke: Constraints rail card, permanent dimensions and constraint symbols on
// a plate like the customer's screenshot (rectangle and slot, 50 mm, 25 mm, R 25 mm, H/V/equal).
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const out='target/0066-browser/rev2-m3';
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
  const tools=page.getByRole('navigation',{name:'Sketch tools'});
  const card=page.getByRole('toolbar',{name:'Constraints'});
  // AC24: eleven constraint tools in their own card, none of them in the geometry card;
  // 0066 revision 4 (AC44) adds Horizontal and Vertical distance.
  assert.equal(await card.getByRole('button').count(),13);
  await page.screenshot({path:`${out}/rail-card.png`,clip:{x:0,y:0,width:240,height:620}});
  const tool=name=>tools.getByRole('button',{name,exact:true});
  const rail=name=>card.locator(`[data-sketch-rail="${name}"]`);
  const notes=page.getByLabel('Sketch annotations');
  const settle=()=>page.waitForTimeout(150);
  // A rectangle: Horizontal and Vertical symbols on its sides.
  await tool('Rectangle').click();
  await page.mouse.click(420,300);await page.mouse.click(620,420);await settle();
  // A slot to its right: equal and tangent symbols, R and spacing dimensions.
  await tool('Slot').click();
  await page.mouse.click(780,360);await page.mouse.click(940,360);await page.mouse.click(940,410);await settle();
  await tool('Select').click();
  assert.ok(await notes.locator('[data-sketch-symbol="horizontal"]').count()>=2,'horizontal symbols');
  assert.ok(await notes.locator('[data-sketch-symbol="vertical"]').count()>=2,'vertical symbols');
  assert.equal(await notes.locator('[data-sketch-symbol="equal"]').count(),2,'equal symbols on both caps');
  assert.equal(await notes.locator('[data-sketch-symbol="tangent"]').count(),4,'tangent symbols at the joints');
  await notes.getByRole('button',{name:/^Radius R /}).waitFor();
  // Selection first: the top edge selected, Dimension applies at once (0066 revision 3) with the
  // measured length and opens the box on it; the typed value replaces it.
  await page.mouse.click(520,300);await settle();
  // A segment starts a point-on Coincident (0066 AC37): the tool is available.
  assert.equal(await rail('coincident').getAttribute('aria-disabled'),null,'coincident takes a segment and a point');
  await rail('dimension').click();
  const entry=page.getByRole('group',{name:'Dimension',exact:true});
  await entry.waitFor();
  await page.keyboard.type('50');await page.keyboard.press('Enter');
  await notes.getByRole('button',{name:'Length 50 mm'}).waitFor();
  await page.keyboard.press('Escape');await page.keyboard.press('Escape');
  // The left edge: 25 mm.
  await page.mouse.click(420,360);await settle();
  const left=await page.evaluate(()=>[...document.querySelectorAll('[data-sketch-dimension-text]')].length);
  await rail('dimension').click();
  await entry.waitFor();
  await page.waitForFunction(n=>document.querySelectorAll('[data-sketch-dimension-text]').length>n,left);
  await page.keyboard.type('25');await page.keyboard.press('Enter');
  await notes.getByRole('button',{name:'Length 25 mm'}).waitFor();
  await page.keyboard.press('Escape');await page.keyboard.press('Escape');
  // Double-click the slot radius text: the entry box opens at the text; 25 makes it R 25 mm.
  const radius=notes.getByRole('button',{name:/^Radius R /});
  const at=await radius.boundingBox();
  await page.mouse.dblclick(at.x+at.width/2,at.y+at.height/2);
  const edit=page.getByRole('group',{name:'Dimension',exact:true});
  await edit.waitFor();
  await page.screenshot({path:`${out}/edit-radius-entry.png`});
  await page.keyboard.press('Control+a');await page.keyboard.type('25');await page.keyboard.press('Enter');
  await notes.getByRole('button',{name:'Radius R 25 mm'}).waitFor();
  const texts=await notes.locator('[data-sketch-dimension-text]').allInnerTexts();
  assert.ok(texts.includes('50 mm'),JSON.stringify(texts));
  assert.ok(texts.includes('25 mm'),JSON.stringify(texts));
  assert.ok(texts.includes('R 25 mm'),JSON.stringify(texts));
  await page.mouse.move(1200,800);await settle();
  await page.screenshot({path:`${out}/plate-dimensions-symbols.png`});
  // Clicking a symbol selects its constraint (AC22, AC32).
  const symbol=await notes.locator('[data-sketch-symbol="equal"]').first().boundingBox();
  await page.mouse.click(symbol.x+symbol.width/2,symbol.y+symbol.height/2);await settle();
  assert.equal(await notes.locator('[data-sketch-symbol="equal"]').first().evaluate(e=>getComputedStyle(e).backgroundColor),'rgb(47, 111, 238)');
  await page.screenshot({path:`${out}/symbol-selected.png`});
  assert.deepEqual(errors,[]);
  console.log('rev2-m3 smoke passed',JSON.stringify(texts));
 } finally { await browser.close(); }
})().catch(e=>{console.error(e);process.exit(1);});

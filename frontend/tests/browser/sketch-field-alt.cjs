// Alt in a focused numeric entry field on the revised interaction (0066 revision 2, AC34): Alt typed
// into the on-canvas entry box is text/system input, never a mode change and never a Vector snapping
// inversion. The pending preview, the draft and the focus stay exactly as they were, both with
// Vector snapping stored on and off, and the typed dimensions commit exactly.
const assert=require('node:assert/strict');
const fs=require('node:fs');
const h=require('./sketch-rev2-support.cjs'),geometry=require('./sketch-geometry-oracles.cjs');
(async()=>{
 fs.mkdirSync('target/0066-browser',{recursive:true});
 const browser=await h.launch();
 try{for(const stored of [false,true]){
  const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[];page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(8000);
  await page.goto(h.url(),{waitUntil:'networkidle'});
  await page.getByRole('tab',{name:'Snapping',exact:true}).click();
  if(stored)await page.getByRole('switch',{name:'Snapping',exact:true}).click();
  for(const target of ['Objects','Guides','Nodes'])await page.getByRole('button',{name:target,exact:true}).click();
  const frame=await page.locator('[data-page-frames] > div').first().boundingBox();
  await h.menu(page,'Workbench','Sketch');
  // Polyline: Alt cycles the mode on the canvas, but not while the entry field has focus.
  await h.tool(page,'Polyline');await page.mouse.click(350,300);await page.mouse.move(430,340);
  assert.equal(await h.alt(page),'Perpendicular line','Alt on the canvas cycles');
  await h.alt(page,5);
  assert.equal(await page.locator('[data-sketch-mode-label]').textContent(),'Line','six presses cycle all six modes back to Line');
  await page.keyboard.press('Escape');await page.keyboard.press('Escape');
  await h.tool(page,'Polyline');await page.mouse.click(350,300);await page.mouse.move(430,340);await page.keyboard.press('Tab');
  const length=page.getByRole('textbox',{name:'Sketch Length',exact:true});await length.fill('30 mm');
  const labelBefore=await page.locator('[data-sketch-mode-label]').textContent();
  await length.press('Alt');await h.pump(page);
  assert.equal(await length.inputValue(),'30 mm','Alt keeps the draft');assert(await length.evaluate(e=>e===document.activeElement),'Alt keeps the focus');
  assert.equal(await page.locator('[data-sketch-mode-label]').textContent(),labelBefore,'Alt in a field is no mode change');
  await page.keyboard.press('Escape');await page.keyboard.press('Escape');
  // Rectangle: the pending preview does not change while Alt is tapped or held in the field.
  await h.tool(page,'Rectangle');await page.mouse.click(350,300);
  await page.mouse.move(frame.x+frame.width/2+4,frame.y+frame.height/2+4);
  await page.keyboard.press('Tab');
  const width=page.getByRole('textbox',{name:'Sketch Width',exact:true});await width.fill('30 mm');await width.focus();await h.pump(page);
  const clip={x:300,y:180,width:650,height:600};
  const before=await page.screenshot({clip,caret:'hide'});
  await width.press('Alt');await h.pump(page);
  assert.equal(await width.inputValue(),'30 mm');assert(await width.evaluate(e=>e===document.activeElement));
  await page.keyboard.down('Alt');await h.pump(page);
  const held=await page.screenshot({clip,caret:'hide'});assert(before.equals(held),'Alt held in the focused field leaves the Sketch preview unchanged');
  await page.keyboard.up('Alt');await h.pump(page);
  const restored=await page.screenshot({clip,caret:'hide'});assert(before.equals(restored),'Alt release leaves the preview unchanged');
  assert.equal(await width.inputValue(),'30 mm');
  await page.getByRole('textbox',{name:'Sketch Height',exact:true}).fill('20 mm');await page.keyboard.press('Enter');
  const saved=await h.save(page,'target/0066-browser',`focused-alt-${stored}`);
  geometry('typed-rectangle',saved.graph);
  assert.equal(saved.json.snapping.enabled,stored);
  assert.deepEqual(errors,[]);await page.close();
 }
 console.log('PASS Alt in the focused entry field: no mode change, draft/focus/preview unchanged, typed dimensions commit exactly; both stored snapping states');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exit(1);});

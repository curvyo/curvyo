// Production host regression for the page-owned workbench on the revised interaction (0066
// revisions 1 and 2): Radius preview, Reference, geometric action after Reference, editable entry
// fields, Tab/Escape, Arc variant by Alt and right-click cancel, empty-page draft isolation and the
// parameter-first fx tab. There is no options bar: numeric entry is the on-canvas box.
const assert=require('node:assert/strict');
const fs=require('node:fs');
const h=require('./sketch-rev2-support.cjs');
(async()=>{
 fs.mkdirSync('target/0066-browser',{recursive:true});
 const browser=await h.launch();
 try {
  const {page,errors}=await h.fresh(browser);
  const label=()=>page.locator('[data-sketch-mode-label]').evaluate(e=>e.firstChild.textContent);
  assert.equal(await page.getByRole('button',{name:'New',exact:true}).count(),0);
  assert.equal(await page.getByRole('form',{name:'Sketch tool options'}).count(),0,'no options bar');
  await h.tool(page,'Circle');await page.mouse.click(400,300);
  await h.typeEntry(page,[['Sketch Radius','6 mm']]);
  // Nothing selected: Dimension arms (a selection would apply at once, 0066 revision 3); pick the circle on its outline.
  await h.rail(page,'dimension').click();await page.mouse.click(423,300);await page.mouse.move(520,360);
  await page.keyboard.press('Alt'); // Diameter (default) -> Radius
  await page.mouse.move(520,361);
  await page.waitForFunction(()=>document.querySelector('[aria-label="Sketch annotations"]')?.innerText.includes('6.00 mm'));
  assert(!(await h.notes(page).innerText()).includes('12.00 mm'),'preview reflects Radius rather than a stale Diameter');
  await page.mouse.click(520,361);
  const entry=page.getByRole('group',{name:'Dimension',exact:true});await entry.waitFor();
  await entry.getByRole('checkbox',{name:'Reference',exact:true}).check();
  assert.equal(await entry.getByRole('textbox',{name:'Sketch Radius',exact:true}).inputValue().then(v=>Math.abs(parseFloat(v)-6)<1e-6),true,'a reference shows the measured radius');
  // Ticking Reference leaves the focus on the checkbox, where Enter does nothing (reported finding): refocus the field.
  await entry.getByRole('textbox',{name:'Sketch Radius',exact:true}).focus();
  await page.keyboard.press('Enter');
  await page.getByLabel('Sketch relationships').getByRole('button',{name:/Radius · Reference/}).waitFor();
  assert.equal(await page.getByLabel('Sketch relationships').getByRole('button',{name:/Radius · Reference/}).count(),1);
  // A Reference preference must not make a later geometric action a reference constraint.
  await page.keyboard.press('Escape');await page.keyboard.press('Escape');
  await (await h.geometry(page)).getByRole('button',{name:/^circle /}).click();
  // The typed radius 6 is a driving dimension (AC39): Fixed on the circle is redundant with it ('Fixed · Redundant').
  await h.rail(page,'fixed').click();
  await page.getByLabel('Sketch relationships').getByRole('button',{name:/^Fixed/}).waitFor();
  assert.equal(await page.getByLabel('Sketch relationships').getByRole('button',{name:/^Fixed/}).count(),1);
  // Drawing inputs remain editable after Reference dimensioning.
  await h.tool(page,'Point');await page.mouse.move(600,500);await page.keyboard.press('Tab');
  const box=page.getByRole('group',{name:'Point',exact:true});await box.waitFor();
  const x=box.getByRole('textbox',{name:'Sketch X',exact:true});
  assert.equal(await x.evaluate(e=>e.readOnly),false);await x.fill('25 mm');
  await box.getByRole('textbox',{name:'Sketch Y',exact:true}).fill('15 mm');await page.keyboard.press('Enter');
  await h.tool(page,'Polyline');await page.mouse.click(600,450);await page.mouse.move(680,470);
  await page.keyboard.press('Tab');
  assert.equal(await page.evaluate(()=>document.activeElement?.getAttribute('aria-label')),'Sketch Length');
  await page.getByRole('textbox',{name:'Sketch Length',exact:true}).fill('30 mm');
  await page.keyboard.press('Escape');await page.keyboard.press('Escape');
  assert.equal(await page.locator('[data-sketch-entry]').count(),0);
  // Arc: Alt picks the variant before the first pick; after it Alt only hints; right-click cancels.
  await h.tool(page,'Arc');await page.mouse.move(650,500);await h.pump(page);
  assert.equal(await label(),'Arc, 3 points');
  assert.equal(await h.alt(page),'Arc, centre');
  await page.mouse.click(650,500);await h.pump(page);
  await page.keyboard.press('Alt');await h.pump(page);
  assert.equal(await label(),'Arc, centre','after a pick Alt keeps the variant (AC35)');
  assert.match(await page.locator('[data-sketch-mode-label]').innerText(),/Escape/,'the hint names Escape');
  await page.mouse.click(700,500,{button:'right'});await h.pump(page);
  assert.equal(await page.getByText(/Finish the shape or press Escape/).count(),0,'right-click cancels the pending Arc and its hint');
  assert.equal(await page.getByLabel('Sketch entities').getByRole('button',{name:/^arc /}).count(),0,'no arc was created');
  await page.screenshot({path:'target/0066-browser/revision-dimension.png'});
  // Two virtual page contexts both have no Sketch; their entry drafts still differ.
  await page.getByRole('tab',{name:'Document',exact:true}).click();
  await page.getByRole('button',{name:'Add page',exact:true}).click();
  await h.tool(page,'Point');await page.mouse.move(500,400);await page.keyboard.press('Tab');
  await page.getByRole('textbox',{name:'Sketch X',exact:true}).fill('123 mm');
  await page.getByRole('button',{name:'Add page',exact:true}).click();
  assert.equal(await page.locator('[data-sketch-entry]').count(),0,'the draft does not follow a new page');
  await page.mouse.move(500,400);await h.focusCanvas(page);await page.keyboard.press('Tab');
  assert.notEqual(await page.getByRole('textbox',{name:'Sketch X',exact:true}).inputValue(),'123 mm');
  await page.keyboard.press('Escape');
  await page.getByRole('tab',{name:'Parameters',exact:true}).click();
  await page.getByRole('button',{name:'Add parameter',exact:true}).click();
  await page.getByRole('textbox',{name:'New parameter name',exact:true}).fill('width');
  await page.getByRole('textbox',{name:'New parameter expression',exact:true}).fill('80 mm');
  await page.getByRole('button',{name:'Add',exact:true}).click();
  assert.equal(await page.getByRole('textbox',{name:'width expression',exact:true}).inputValue(),'80 mm');
  assert.deepEqual(errors,[]);
  await page.screenshot({path:'target/0066-browser/revision-host.png'});
  console.log('PASS: Radius preview/reference, geometric action, editable entry fields, Tab/Escape, Arc Alt variant/hint and right-click cancel, empty-page draft isolation and parameter-first fx');
 } finally {await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});

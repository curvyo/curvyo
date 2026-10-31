// Independent real-event acceptance on the revised interaction (0066 revisions 1 and 2): dimension
// typed in the on-canvas entry box, double-click edit at the text, Properties field commit/refusal,
// native text undo, free-centre status, conflict repair and the saved artifact.
// Run from the repository root with Playwright on NODE_PATH.
const assert=require('node:assert/strict');
const fs=require('node:fs');
const h=require('./sketch-rev2-support.cjs');
(async()=>{
 fs.mkdirSync('target/0066-browser',{recursive:true});
 const browser=await h.launch();
 try{
  const {page,errors}=await h.fresh(browser);
  await h.tool(page,'Circle');await page.mouse.click(450,300);await page.mouse.click(500,300);
  const circle=(await h.geometry(page)).getByRole('button',{name:/circle/});
  await circle.click();
  await h.rail(page,'dimension').click();await page.mouse.move(560,360);await page.mouse.click(560,360);
  const entry=page.getByRole('group',{name:'Dimension',exact:true});await entry.waitFor();
  await entry.getByRole('textbox',{name:'Sketch Diameter',exact:true}).fill('20 mm');await page.keyboard.press('Enter');
  await h.notes(page).locator('[data-sketch-dimension-text]').filter({hasText:/^⌀ 20 mm$/}).waitFor();
  await page.keyboard.press('Escape');await page.keyboard.press('Escape');
  assert(!(await circle.textContent()).includes('Fully constrained'),'fixed diameter leaves centre free');
  assert.match(await page.getByLabel('Sketch status').textContent(),/Under-constrained/);
  await h.tool(page,'Select');
  // Double-click on the dimension text opens the entry box at the text (AC31, AC33).
  const text=h.notes(page).locator('[data-sketch-dimension-text]').first();
  const at=await text.boundingBox();await page.mouse.dblclick(at.x+at.width/2,at.y+at.height/2);
  const editBox=page.getByRole('group',{name:'Dimension',exact:true});await editBox.waitFor();
  const near=await editBox.boundingBox();
  assert(Math.hypot(near.x-at.x,near.y-at.y)<200,'entry box sits at the dimension text');
  await editBox.getByRole('textbox').first().fill('24 mm');await page.keyboard.press('Enter');
  await h.notes(page).locator('[data-sketch-dimension-text]').filter({hasText:/^⌀ 24 mm$/}).waitFor();
  // Properties field: commit, refusal and Escape restore.
  await page.getByRole('tab',{name:'Sketches',exact:true}).click();
  const field=page.getByLabel('Diameter expression',{exact:true});
  await page.getByRole('button',{name:'Edit diameter',exact:true}).click();
  assert.equal(await field.inputValue(),'24 mm');
  await field.fill('missing_parameter');await field.press('Enter');
  assert.equal(await field.getAttribute('aria-invalid'),'true');
  await field.press('Escape');
  await page.getByRole('button',{name:'Edit diameter',exact:true}).click();
  assert.equal(await field.inputValue(),'24 mm');
  // Native text undo changes the draft; it must not undo the committed dimension.
  await field.press('Control+a');await field.pressSequentially('27 mm');
  await field.press('Control+z');await field.press('Escape');
  await page.getByRole('button',{name:'Edit diameter',exact:true}).click();
  assert.equal(await field.inputValue(),'24 mm');await field.press('Escape');
  // A second, conflicting diameter: Conflicting until the offending dimension is removed.
  await circle.click();
  await h.rail(page,'dimension').click();await page.mouse.move(600,400);await page.mouse.click(600,400);
  await entry.waitFor();
  await entry.getByRole('textbox',{name:'Sketch Diameter',exact:true}).fill('30 mm');await page.keyboard.press('Enter');
  await page.waitForFunction(()=>/Conflicting/.test(document.querySelector('[aria-label="Sketch status"]')?.textContent||''));
  await page.keyboard.press('Escape');await page.keyboard.press('Escape');
  await page.getByRole('button',{name:'Remove diameter',exact:true}).last().click();
  assert.match(await page.getByLabel('Sketch status').textContent(),/Under-constrained/);
  await page.screenshot({path:'target/0066-browser/independent-circle-repair.png'});
  await h.save(page,'target/0066-browser','independent-circle');
  assert.deepEqual(errors,[]);
  console.log('PASS on-canvas dimension, double-click edit at the text, field commit/refusal/native undo, free-centre status, conflict repair and saved artifact');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exit(1);});

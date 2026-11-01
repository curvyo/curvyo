// 0066 Sketch workbench journey on the revised interaction (customer revisions 1 to 5): rectangle,
// formulas, dimensions typed in the on-canvas entry box, field drafts, conversion and undo.
// The rail applies Dimension to a fitting selection at once and opens its entry box (AC40).
// Run from the repository root with the isolated Playwright runtime on NODE_PATH.
const assert=require('node:assert/strict');
const fs=require('node:fs');
const h=require('./sketch-rev2-support.cjs');
(async()=>{
 fs.mkdirSync('target/0066-browser',{recursive:true});
 const browser=await h.launch();
 try{
 const {page,errors}=await h.fresh(browser);
 assert.equal(await page.getByRole('form',{name:'Sketch tool options'}).count(),0,'no options bar (AC34)');
 assert.equal(await page.getByRole('button',{name:'New',exact:true}).count(),0,'no New: the first geometry creates the page Sketch');
 await h.tool(page,'Rectangle');
 await page.mouse.move(350,200);await page.mouse.down();await page.mouse.move(650,400,{steps:8});await page.mouse.up();
 assert.equal(await (await h.geometry(page)).getByRole('button').count(),9);
 // Named parameter first (dedicated fx tab), then used by a length dimension typed on the canvas.
 await page.getByRole('tab',{name:'Parameters',exact:true}).click();
 await page.getByRole('button',{name:'Add parameter',exact:true}).click();
 await page.getByLabel('New parameter name').fill('plate_width');await page.getByLabel('New parameter expression').fill('80 mm');
 await page.getByRole('button',{name:'Add',exact:true}).click();
 await (await h.geometry(page)).getByRole('button',{name:/line/}).first().click();
 await h.rail(page,'dimension').click(); // applies at once; the entry box opens on it (AC40)
 const box=page.getByRole('group',{name:'Dimension',exact:true});await box.waitFor();
 await box.getByRole('textbox').first().fill('plate_width');await page.keyboard.press('Enter');
 await page.getByLabel('Sketch annotations').getByRole('button',{name:/^Length /}).waitFor();
 await page.keyboard.press('Escape');await page.keyboard.press('Escape');
 // A circle and its Diameter dimension (a circle's smart dimension starts as Diameter; Alt switches to Radius, AC34).
 await h.tool(page,'Circle');await page.mouse.click(450,280);await page.mouse.click(480,280);
 await (await h.geometry(page)).getByRole('button',{name:/circle/}).click();
 await h.rail(page,'dimension').click();
 await box.waitFor();
 await box.getByRole('textbox',{name:'Sketch Diameter',exact:true}).fill('12');await page.keyboard.press('Enter');
 await page.getByLabel('Sketch annotations').locator('[data-sketch-dimension-text]').filter({hasText:/^\u2300 12 mm$/}).first().waitFor();
 await page.keyboard.press('Escape');await page.keyboard.press('Escape');
 await h.tool(page,'Select');
 // Parameter fields: Escape restores the committed formula, an edit changes the dimension.
 await page.getByRole('tab',{name:'Parameters',exact:true}).click();
 await page.getByLabel('plate_width expression').fill('90 mm');
 await page.getByLabel('plate_width expression').press('Escape');
 assert.equal(await page.getByLabel('plate_width expression').inputValue(),'80 mm');
 await page.getByRole('tab',{name:'Sketches',exact:true}).click();
 await page.getByRole('button',{name:'Edit length',exact:true}).click();
 await page.waitForFunction(()=>document.activeElement?.getAttribute('aria-label')==='Length expression');
 await page.getByLabel('Length expression').fill('99');await page.getByLabel('Length expression').press('Escape');
 await page.getByRole('button',{name:'Edit length',exact:true}).click();
 await page.waitForFunction(()=>document.activeElement?.getAttribute('aria-label')==='Length expression');
 assert.equal(await page.getByLabel('Length expression').inputValue(),'plate_width');
 await page.getByLabel('Length expression').press('Escape');
 // Parameter draft cancellation.
 await page.getByRole('tab',{name:'Parameters',exact:true}).click();
 await page.getByRole('button',{name:'Add parameter',exact:true}).click();await page.getByLabel('New parameter name').fill('discard');await page.getByLabel('New parameter name').press('Escape');
 await page.waitForFunction(()=>document.activeElement?.textContent==='Add parameter');
 await page.getByRole('button',{name:'Add parameter',exact:true}).click();assert.equal(await page.getByLabel('New parameter name').inputValue(),'');await page.getByRole('button',{name:'Cancel',exact:true}).click();
 await page.getByRole('tab',{name:'Sketches',exact:true}).click();
 await page.screenshot({path:'target/0066-browser/plate.png'});
 // A numeric entry draft belongs to the pending segment and is gone after a page switch.
 await h.tool(page,'Polyline');await page.mouse.click(300,700);await page.mouse.move(380,720);await page.keyboard.press('Tab');
 await page.locator('[data-sketch-entry]').waitFor();await page.getByRole('textbox',{name:'Sketch Length',exact:true}).fill('77');
 await page.keyboard.press('Escape');await page.keyboard.press('Escape');
 assert.equal(await page.locator('[data-sketch-entry]').count(),0,'Escape closes the entry and discards the draft');
 await h.tool(page,'Select');
 // Conversion leaves the Sketch workbench and is undoable.
 await page.getByRole('button',{name:'Convert to paths',exact:true}).click();
 await page.getByRole('button',{name:'Workbench',exact:true}).waitFor();
 await page.keyboard.press('Control+z');
 await h.menu(page,'Workbench','Sketch');
 assert.ok(await page.getByRole('navigation',{name:'Sketch tools'}).isVisible());
 // Properties panel hide/show survives a workbench change.
 await page.getByRole('button',{name:'Hide panel',exact:true}).click();
 await h.menu(page,'Workbench','Vector');
 assert.ok(await page.getByRole('button',{name:'Show panel',exact:true}).isVisible());
 await h.menu(page,'Workbench','Sketch');
 assert.ok(await page.getByRole('button',{name:'Show panel',exact:true}).isVisible());
 await page.getByRole('button',{name:'Show panel',exact:true}).click();
 await h.tool(page,'Select');
 await page.keyboard.press('p');
 assert.ok(await page.getByRole('navigation',{name:'Sketch tools'}).isVisible());
 assert.equal(await page.getByRole('form',{name:'Sketch tool options'}).count(),0,'still no options bar after a shortcut');
 assert.equal(await page.getByRole('combobox',{name:'Sketch tool variant'}).count(),0,'no variant menu: variants cycle with Alt');
 assert.deepEqual(errors,[]);
 console.log('PASS creation, formulas, on-canvas dimensions, draft cancellation, repeated Edit focus, entry-draft isolation, conversion and undo');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exit(1);});

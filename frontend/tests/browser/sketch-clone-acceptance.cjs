// Linked clone -> Sketch original entry and Radius edit on the revised interaction (0066 revisions
// 1 and 2). The Sketch of a page is addressed by page now: there is no "Active sketch" picker, so
// entering from a selected clone must land in the original's geometry. Verify the saved artifact with
// cargo test --offline -p curvyo-document-core --test acceptance_0066_document d12_actual_browser -- --ignored --nocapture
const assert=require('node:assert/strict');
const h=require('./sketch-rev2-support.cjs');
(async()=>{
 const browser=await h.launch();
 const {page,errors}=await h.fresh(browser);
 try{
 await h.tool(page,'Circle');await page.mouse.click(400,300);
 await h.typeEntry(page,[['Sketch Radius','10 mm']]);
 const first=await (await h.geometry(page)).getByRole('button',{name:/^circle /}).count();
 assert.equal(first,1,'the original Sketch holds one circle');
 await h.menu(page,'Workbench','Vector');
 await page.getByRole('button',{name:'Linked clone',exact:true}).click();
 await h.menu(page,'Workbench','Sketch');
 const circles=await (await h.geometry(page)).getByRole('button',{name:/^circle /}).count();
 assert.equal(circles,1,'selected clone enters the original constraint system');
 // The typed radius 10 is already a driving R 10 (AC39); change it through that dimension.
 const text=h.notes(page).locator('[data-sketch-dimension-text]').filter({hasText:/^R 10 mm$/});
 await text.waitFor();const at=await text.boundingBox();
 await page.mouse.dblclick(at.x+at.width/2,at.y+at.height/2);
 const entry=page.getByRole('group',{name:'Dimension',exact:true});await entry.waitFor();
 await entry.getByRole('textbox',{name:'Sketch Radius',exact:true}).fill('15 mm');await page.keyboard.press('Enter');
 await h.notes(page).locator('[data-sketch-dimension-text]').filter({hasText:/^R 15 mm$/}).waitFor();
 await h.save(page,'target/0066-browser','clone-entry');
 assert.deepEqual(errors,[]);console.log('PASS actual Linked clone selection enters original and edits Radius; saved geometry verifier required');
 }catch(e){console.log(await page.locator('body').innerText());throw e;}finally{await browser.close();}
})().catch(e=>{console.error(e);process.exit(1);});

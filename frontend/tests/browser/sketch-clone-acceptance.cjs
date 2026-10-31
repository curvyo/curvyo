const {chromium}=require('playwright'),assert=require('node:assert/strict');
(async()=>{
 const browser=await chromium.launch({headless:true,args:['--no-sandbox','--enable-unsafe-webgpu','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
 const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[];page.on('pageerror',e=>errors.push(e.stack));
 try{
 await page.goto(process.env.CURVYO_BROWSER_URL||'http://127.0.0.1:5173/',{waitUntil:'networkidle'});
 await page.getByLabel('Workbench',{exact:true}).selectOption('sketch');await page.getByRole('button',{name:'New',exact:true}).click();
 const original=await page.getByLabel('Active sketch').inputValue();
 await page.getByRole('button',{name:'Circle',exact:true}).click();await page.mouse.click(400,300);
 await page.getByLabel('Sketch Radius',{exact:true}).fill('10 mm');await page.getByLabel('Sketch tool options').getByRole('button',{name:'Apply',exact:true}).click();
 await page.getByLabel('Workbench',{exact:true}).selectOption('vector');
 await page.getByRole('button',{name:'Linked clone',exact:true}).click();
 await page.getByLabel('Workbench',{exact:true}).selectOption('sketch');
 assert.equal(await page.getByLabel('Active sketch').inputValue(),original,'selected clone enters original constraint system');
 await page.getByText(/^Geometry \(/).click();await page.getByLabel('Sketch entities').getByRole('button',{name:/^circle /}).click();
 await page.getByLabel('Constraint actions').getByRole('button',{name:'Radius',exact:true}).click();
 await page.getByLabel('New radius').fill('15 mm');await page.getByLabel('Constraint actions').getByRole('button',{name:'Apply',exact:true}).click();
 await page.getByRole('tab',{name:/^File/}).click();const pending=page.waitForEvent('download');await page.getByRole('button',{name:'Save',exact:true}).click();
 await(await pending).saveAs('target/0066-browser/clone-entry.curvyo');assert.deepEqual(errors,[]);console.log('PASS actual Linked clone selection enters original and edits Radius; saved geometry verifier required');
 }catch(e){console.log(await page.locator('body').innerText());throw e;}finally{await browser.close();}
})();

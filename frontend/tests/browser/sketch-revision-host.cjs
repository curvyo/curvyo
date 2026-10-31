// Production host regression for the revised page-owned workbench.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
(async()=>{
 fs.mkdirSync('target/0066-browser',{recursive:true});
 const browser=await chromium.launch({headless:true,args:['--no-sandbox','--enable-unsafe-webgpu','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
 try {
  const page=await browser.newPage({viewport:{width:1440,height:1000}}), errors=[];
  page.on('pageerror',e=>errors.push(e.stack));
  page.setDefaultTimeout(10000);
  await page.goto(process.env.CURVYO_BROWSER_URL||'http://127.0.0.1:1421/',{waitUntil:'networkidle'});
  const menu=async(name,choice)=>{await page.getByRole('button',{name,exact:true}).click();await page.getByRole('menuitemradio',{name:choice,exact:true}).click();};
  const tool=async(name)=>page.getByRole('navigation',{name:'Sketch tools'}).getByRole('button',{name,exact:true}).click();
  const options=page.getByRole('form',{name:'Sketch tool options'});
  await menu('Workbench','Sketch');
  assert.equal(await page.getByRole('button',{name:'New',exact:true}).count(),0);
  await tool('Circle');await page.mouse.click(400,300);
  await page.getByRole('textbox',{name:'Sketch Radius',exact:true}).fill('6 mm');
  await options.getByRole('button',{name:'Apply',exact:true}).click();
  await page.getByRole('tab',{name:'Sketches',exact:true}).click();
  await page.getByText(/^Geometry \(/).click();
  await page.getByLabel('Sketch entities').getByRole('button',{name:/^circle /}).click();
  await tool('Dimension');await page.mouse.move(520,360);
  await menu('Circle dimension','Radius');await page.mouse.move(520,360);
  await page.waitForFunction(()=>document.body.innerText.includes('6.00 mm'));
  assert(!(await page.locator('body').innerText()).includes('12.00 mm'),'preview reflects Radius rather than stale Diameter');
  await page.mouse.click(520,360);
  await options.getByRole('checkbox',{name:'Reference',exact:true}).check();
  assert(await page.getByRole('textbox',{name:'Sketch Radius',exact:true}).evaluate(e=>e.readOnly));
  await options.getByRole('button',{name:'Apply',exact:true}).click();
  assert.equal(await page.getByLabel('Sketch relationships').getByRole('button',{name:/Radius · Reference/}).count(),1);
  // Reference preference must not make a later geometric action a reference constraint.
  await tool('Constraints');await menu('Constraint tool','Fixed');
  await options.getByRole('button',{name:'Apply',exact:true}).click();
  assert.equal(await page.getByLabel('Sketch relationships').getByRole('button',{name:'Fixed',exact:true}).count(),1);
  // Drawing inputs remain editable after Reference dimensioning.
  await tool('Point');const x=page.getByRole('textbox',{name:'Sketch X',exact:true});
  assert.equal(await x.evaluate(e=>e.readOnly),false);await x.fill('25 mm');
  await page.getByRole('textbox',{name:'Sketch Y',exact:true}).fill('15 mm');
  await options.getByRole('button',{name:'Apply',exact:true}).click();
  await tool('Polyline');await page.mouse.click(600,450);await page.mouse.move(680,470);
  await page.keyboard.press('Tab');
  assert.equal(await page.evaluate(()=>document.activeElement?.getAttribute('aria-label')),'Sketch Length');
  await page.getByRole('textbox',{name:'Sketch Length',exact:true}).fill('30 mm');
  await page.keyboard.press('Escape');await page.keyboard.press('Escape');
  await tool('Arc');await page.mouse.click(650,500);await page.keyboard.press('Alt');
  assert.equal(await page.getByRole('button',{name:'Sketch tool variant',exact:true}).innerText(),'Centre, start, end');
  await page.mouse.click(700,500,{button:'right'});
  await page.screenshot({path:'target/0066-browser/revision-dimension.png'});
  // Two virtual page contexts both have active=null; their numeric drafts still differ.
  await page.getByRole('tab',{name:'Document',exact:true}).click();
  await page.getByRole('button',{name:'Add page',exact:true}).click();
  await tool('Point');await page.getByRole('textbox',{name:'Sketch X',exact:true}).fill('123 mm');
  await page.getByRole('button',{name:'Add page',exact:true}).click();
  assert.equal(await page.getByRole('textbox',{name:'Sketch X',exact:true}).inputValue(),'');
  await page.getByRole('tab',{name:'Parameters',exact:true}).click();
  await page.getByRole('button',{name:'Add parameter',exact:true}).click();
  await page.getByRole('textbox',{name:'New parameter name',exact:true}).fill('width');
  await page.getByRole('textbox',{name:'New parameter expression',exact:true}).fill('80 mm');
  await page.getByRole('button',{name:'Add',exact:true}).click();
  assert.equal(await page.getByRole('textbox',{name:'width expression',exact:true}).inputValue(),'80 mm');
  assert.deepEqual(errors,[]);
  await page.screenshot({path:'target/0066-browser/revision-host.png'});
  console.log('PASS: Radius preview/reference, geometric action, editable drawing fields, Tab/Escape, Arc Alt/right-click, empty-page draft isolation and parameter-first fx');
 } finally {await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});

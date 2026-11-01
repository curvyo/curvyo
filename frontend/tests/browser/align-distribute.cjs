// Browser acceptance of `specs/0061-align-and-distribute/` criteria 14 to 22:
// align and distribute relative to the active page, and Arrange in a grid,
// pressed through the Select bar of the running dev server. Positions are read
// from the saved project. Screenshots go to target/0061-browser/.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {execFileSync}=require('node:child_process');
const fs=require('node:fs');
const path=require('node:path');
const out=name=>path.resolve('target/0061-browser',name);
fs.mkdirSync(out(''),{recursive:true});
const pump=page=>page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
const near=(actual,expected,what)=>assert(Math.abs(actual-expected)<1e-6,`${what}: ${actual} != ${expected}`);
const shapes=nodes=>nodes.flatMap(n=>n.shape==='layer'||n.shape==='group'?shapes(n.objects||[]):[n]);
let saves=0;
async function saved(page){
  await page.getByRole('tab',{name:/^File/}).click();
  const download=page.waitForEvent('download');
  await page.getByRole('button',{name:'Save',exact:true}).click();
  const file=out(`state-${++saves}.curvyo`);
  await(await download).saveAs(file);
  const data=JSON.parse(execFileSync('python3',['-c','import zipfile,sys; print(zipfile.ZipFile(sys.argv[1]).read("document.json").decode())',file],{encoding:'utf8'}));
  const boxes=shapes(data.objects).map(o=>{const b=o.bounds;return {x0:b.origin.x,y0:b.origin.y,x1:b.origin.x+b.width,y1:b.origin.y+b.height,cx:b.origin.x+b.width/2,cy:b.origin.y+b.height/2,w:b.width,h:b.height};});
  const pages=data.pages.map(p=>({x0:p.origin.x,y0:p.origin.y,x1:p.origin.x+p.size.width,y1:p.origin.y+p.size.height,w:p.size.width,h:p.size.height}));
  return {boxes,pages};
}
async function open(browser){
  const page=await browser.newPage({viewport:{width:1440,height:1000}});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.goto(process.env.CURVYO_BROWSER_URL||'http://127.0.0.1:5173/',{waitUntil:'networkidle'});
  return {page,errors};
}
/** Screen point of document mm on page `index`, read from its frame. */
async function mapper(page,index,doc){
  const frame=await page.locator('[data-page-frames] > div').nth(index).boundingBox();
  const p=doc.pages[index];
  return (x,y)=>({x:frame.x+(x-p.x0)*frame.width/p.w,y:frame.y+(y-p.y0)*frame.height/p.h});
}
async function rect(page,at,a,b){
  await page.getByRole('button',{name:/^Rectangle tool/}).click();
  const p=at(...a),q=at(...b);
  await page.mouse.move(p.x,p.y);await page.mouse.down();await page.mouse.move(q.x,q.y,{steps:4});await page.mouse.up();await pump(page);
}
async function selectTool(page){await page.getByRole('button',{name:/^Select tool/}).click();await pump(page);}
async function nothingSelected(page){await selectTool(page);await page.keyboard.press('Escape');await page.keyboard.press('Escape');await pump(page);}
async function press(page,name){await page.getByRole('button',{name,exact:true}).click();await pump(page);}
async function history(page){
  await page.getByRole('tab',{name:/^History/}).click();await pump(page);
  return page.locator('[role=tabpanel]').innerText();
}
(async()=>{
  const browser=await chromium.launch({headless:true,args:['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader']});

  // Criteria 14, 15, 17: two pages; objects on both; nothing selected aligns
  // only the active page's objects onto the page.
  {
    const {page,errors}=await open(browser);
    let doc=await saved(page);
    await rect(page,await mapper(page,0,doc),[40,40],[60,60]);
    await page.getByRole('tab',{name:/^Document/}).click();
    await page.getByRole('button',{name:'Add page',exact:true}).click();await pump(page);
    doc=await saved(page);
    const on2=await mapper(page,1,doc);
    await rect(page,on2,[doc.pages[1].x0+30,50],[doc.pages[1].x0+60,70]);
    await rect(page,on2,[doc.pages[1].x0+100,120],[doc.pages[1].x0+120,160]);
    await nothingSelected(page);
    const before=(await saved(page)).boxes;
    for(const [name,check] of [
      ['Align left edges',(b,p)=>near(b.x0,p.x0,'left')],
      ['Align bottom edges',(b,p)=>near(b.y1,p.y1,'bottom')],
      ['Align horizontal centres',(b,p)=>near(b.cx,(p.x0+p.x1)/2,'centre')],
    ]){
      await press(page,name);
      doc=await saved(page);
      for(const i of [1,2]) check(doc.boxes[i],doc.pages[1]);
      assert.deepEqual(doc.boxes[0],before[0],`${name}: the other page's object stays`);
    }
    assert.match(await history(page),/Align horizontal centres to page/);
    await page.getByRole('button',{name:'Align left edges',exact:true}).hover();await pump(page);
    await page.screenshot({path:out('page-align-cue.png')});
    // Criterion 14: one object relative to its page; the Pages row activates
    // page 1 and brings it into view, the click selects the object on it.
    await page.getByRole('tab',{name:/^Document/}).click();
    await page.getByRole('grid',{name:'Pages',exact:true}).getByRole('button',{name:'Page 1',exact:true}).click();await pump(page);
    await selectTool(page);
    const centre=(await mapper(page,0,doc))(50,40);await page.mouse.click(centre.x,centre.y);await pump(page);
    await press(page,'Align vertical centres');
    doc=await saved(page);
    near(doc.boxes[0].cy,(doc.pages[0].y0+doc.pages[0].y1)/2,'one object on page 1');
    assert.match(await history(page),/Align vertical centres to page/);
    assert.deepEqual(errors,[]);
    await page.close();
  }

  // Criterion 16: distribute relative to the page with nothing, one and two
  // selected; n + 1 equal gaps.
  {
    const {page,errors}=await open(browser);
    let doc=await saved(page);
    const at=await mapper(page,0,doc);
    await rect(page,at,[20,40],[40,60]);
    await rect(page,at,[90,80],[120,100]);
    await rect(page,at,[150,120],[160,130]);
    await nothingSelected(page);
    await press(page,'Distribute horizontal gaps');
    doc=await saved(page);
    const p=doc.pages[0],b=doc.boxes;
    const gap=(p.w-b[0].w-b[1].w-b[2].w)/4;
    near(b[0].x0,p.x0+gap,'first gap');near(b[1].x0,b[0].x1+gap,'second gap');near(b[2].x0,b[1].x1+gap,'third gap');near(p.x1-b[2].x1,gap,'last gap');
    assert.match(await history(page),/Distribute horizontally on page/);
    await page.screenshot({path:out('page-distribute-nothing.png')});
    // One object: centred across the page.
    await selectTool(page);
    let c=at(b[1].cx,b[1].y0);await page.mouse.click(c.x,c.y);await pump(page);
    await press(page,'Distribute vertical gaps');
    doc=await saved(page);
    near(doc.boxes[1].cy,(p.y0+p.y1)/2,'one object centred down the page');
    // Two objects: thirds.
    c=at(doc.boxes[0].cx,doc.boxes[0].y0);await page.mouse.click(c.x,c.y);
    c=at(doc.boxes[2].cx,doc.boxes[2].y0);await page.keyboard.down('Shift');await page.mouse.click(c.x,c.y);await page.keyboard.up('Shift');await pump(page);
    await press(page,'Distribute horizontal centres');
    doc=await saved(page);
    near(doc.boxes[0].cx,p.x0+p.w/3,'two: first third');near(doc.boxes[2].cx,p.x0+2*p.w/3,'two: second third');
    await page.getByRole('button',{name:'Distribute horizontal centres',exact:true}).hover();await pump(page);
    await page.screenshot({path:out('page-distribute-two.png')});
    assert.deepEqual(errors,[]);
    await page.close();
  }

  // Criteria 18 to 22: six objects of different sizes in 2 rows of 3, gap 5.
  {
    const {page,errors}=await open(browser);
    let doc=await saved(page);
    const at=await mapper(page,0,doc);
    const sizes=[[20,20],[10,30],[24,12],[16,16],[30,10],[12,24]];
    const spots=[[130,30],[20,34],[70,28],[25,120],[120,125],[70,118]];
    for(let i=0;i<6;i++){const [x,y]=spots[i],[w,h]=sizes[i];await rect(page,at,[x,y],[x+w,y+h]);}
    await selectTool(page);await page.keyboard.press('Control+a');await pump(page);
    const grid=page.getByRole('group',{name:'Grid',exact:true});
    await grid.waitFor();
    const rows=grid.getByRole('textbox',{name:'Rows',exact:true}),columns=grid.getByRole('textbox',{name:'Columns',exact:true});
    assert.equal(await rows.getAttribute('placeholder'),'2');assert.equal(await columns.getAttribute('placeholder'),'3');
    // Linked: two columns hold six in three rows.
    await columns.fill('2');assert.equal(await rows.getAttribute('placeholder'),'3');
    // Refused: zero rows; the field is marked and nothing moves.
    await rows.fill('0');await rows.press('Enter');await pump(page);
    assert.equal(await rows.getAttribute('aria-invalid'),'true');
    await grid.getByRole('status').filter({hasText:'Must be 1 or more.'}).waitFor();
    await page.screenshot({path:out('grid-refused.png')});
    await rows.fill('2');await columns.fill('');
    const before=(await saved(page)).boxes;
    await selectTool(page);
    await grid.getByRole('textbox',{name:'Grid gap, millimetres',exact:true}).fill('5');
    await grid.getByRole('textbox',{name:'Grid gap, millimetres',exact:true}).press('Enter');await pump(page);
    doc=await saved(page);
    const cw=Math.max(...before.map(b=>b.w)),ch=Math.max(...before.map(b=>b.h));
    const x0=Math.min(...before.map(b=>b.x0)),y0=Math.min(...before.map(b=>b.y0));
    // Reading order before: top line 1, 2, 0 by x; bottom line 3, 5, 4.
    const order=[1,2,0,3,5,4];
    order.forEach((index,rank)=>{
      const r=Math.floor(rank/3),c=rank%3;
      near(doc.boxes[index].cx,x0+c*(cw+5)+cw/2,`grid x of ${index}`);
      near(doc.boxes[index].cy,y0+r*(ch+5)+ch/2,`grid y of ${index}`);
    });
    assert.match(await history(page),/Arrange in a grid/);
    await page.screenshot({path:out('grid-arranged.png')});
    assert.deepEqual(errors,[]);
    await page.close();
  }
  console.log('PASS browser: page align (two pages, other page untouched, one object), page distribute (n + 1 gaps, one centred, two in thirds), grid (linked fields, refusal, 2 x 3 gap 5 in reading order), history names; no page errors');
  await browser.close();
})().catch(e=>{console.error(e);process.exit(1);});

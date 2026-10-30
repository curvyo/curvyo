const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {execFileSync}=require('node:child_process');
const flatten=nodes=>nodes.flatMap(n=>n.objects?flatten(n.objects):[n]);
(async()=>{
 const browser=await chromium.launch({headless:true,args:['--no-sandbox','--enable-unsafe-webgpu','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
 try {
 let raw;
 for(const stored of [false,true]) for(const [x,y] of [[false,false],[true,true],[true,false],[false,true]]) {
  const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.goto(process.env.CURVYO_BROWSER_URL||'http://127.0.0.1:5173/',{waitUntil:'networkidle'});
  await page.getByRole('tab',{name:'Snapping',exact:true}).click();
  if(stored) await page.getByRole('switch',{name:'Snapping',exact:true}).click();
  for(const target of ['Objects','Guides','Nodes']) await page.getByRole('button',{name:target,exact:true}).click();
  if(!x) await page.getByRole('button',{name:'Horizontal (X)',exact:true}).click();
  if(!y) await page.getByRole('button',{name:'Vertical (Y)',exact:true}).click();
  const frame=await page.locator('[data-page-frames] > div').first().boundingBox();
  await page.getByLabel('Workbench',{exact:true}).selectOption('sketch');
  await page.getByRole('button',{name:'New',exact:true}).click();
  await page.getByRole('button',{name:'Point',exact:true}).click();
  const a={x:frame.x+frame.width/4+4,y:frame.y+frame.height/4+4};
  await page.mouse.move(a.x,a.y);await page.keyboard.down('Alt');
  await page.mouse.click(a.x,a.y);await page.keyboard.up('Alt');
  await page.getByRole('tab',{name:/^File/}).click();
  const download=page.waitForEvent('download');await page.getByRole('button',{name:'Save',exact:true}).click();
  const path=`target/0066-browser/sketch-point-${stored}-${x}-${y}.curvyo`;await(await download).saveAs(path);
  const data=JSON.parse(execFileSync('python3',['-c','import zipfile,sys;print(zipfile.ZipFile(sys.argv[1]).read("document.json").decode())',path],{encoding:'utf8'}));
  const graph=flatten(data.objects).find(n=>n.shape==='sketch').graph;
  const points=Object.entries(graph.entities).filter(([id,e])=>id!==graph.origin&&e.Point).map(([,e])=>e.Point.position);
  assert.equal(points.length,1);const p=points[0];
  if(!x&&!y&&!stored) {raw=p; assert(p.x>53&&p.x<54&&p.y>75&&p.y<76);}
  const expected={x:!stored&&x?52.5:raw.x,y:!stored&&y?74.25:raw.y};
  assert(Math.abs(p.x-expected.x)<1e-6,`${JSON.stringify({stored,x,y,p,expected})}`);
  assert(Math.abs(p.y-expected.y)<1e-6,`${JSON.stringify({stored,x,y,p,expected})}`);
  assert.equal(data.snapping.enabled,stored);assert.equal(data.snapping.x,x);assert.equal(data.snapping.y,y);
  assert.deepEqual(errors,[]);await page.close();
 }
 console.log('PASS actual Sketch Point Alt at same hovered pointer, both stored states, all four axis masks, exact saved positions and unchanged stored settings');
 }finally{await browser.close();}
})();

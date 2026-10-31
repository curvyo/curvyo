// Actual mounted ruler invalidation: pan, zoom, resize and backing-store DPR.
const {chromium}=require('playwright'),assert=require('node:assert/strict');
const pump=page=>page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
async function rulers(page){return page.evaluate(()=>[...document.querySelectorAll('canvas')].filter(c=>c.parentElement?.getAttribute("aria-hidden")==="true"&&c.clientWidth>0&&c.clientHeight>0).map(c=>({width:c.width,height:c.height,cssWidth:c.clientWidth,cssHeight:c.clientHeight,pixels:c.toDataURL()})));}
(async()=>{
 const browser=await chromium.launch({headless:true,args:['--no-sandbox','--enable-unsafe-webgpu','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
 try{for(const dpr of [1,2]){
  const page=await browser.newPage({viewport:{width:1440,height:1000},deviceScaleFactor:dpr});
  await page.goto(process.env.CURVYO_BROWSER_URL||'http://127.0.0.1:1421/',{waitUntil:'networkidle'});
  await page.mouse.move(1200,800);await pump(page);
  const initial=await rulers(page);assert.equal(initial.length,2,JSON.stringify(initial.map(({pixels,...r})=>r)));
  for(const r of initial){assert.equal(r.width,Math.round(r.cssWidth*dpr));assert.equal(r.height,Math.round(r.cssHeight*dpr));}
  await page.mouse.move(500,400);await page.mouse.down({button:'middle'});await page.mouse.move(550,430,{steps:3});await page.mouse.up({button:'middle'});
  await page.mouse.move(1200,800);await pump(page);
  const panned=await rulers(page);assert(panned.some((r,i)=>r.pixels!==initial[i].pixels),'mounted rulers repaint after pan');
  await page.mouse.move(500,400);await page.keyboard.down('Control');await page.mouse.wheel(0,-200);await page.keyboard.up('Control');await page.mouse.move(1200,800);await pump(page);
  const zoomed=await rulers(page);assert(zoomed.some((r,i)=>r.pixels!==panned[i].pixels),'mounted rulers repaint after zoom');
  await page.setViewportSize({width:1600,height:1050});await pump(page);
  const resized=await rulers(page);assert(resized.some((r,i)=>r.width!==zoomed[i].width||r.height!==zoomed[i].height),'mounted rulers resize backing store');
  for(const r of resized){assert.equal(r.width,Math.round(r.cssWidth*dpr));assert.equal(r.height,Math.round(r.cssHeight*dpr));}
  await page.screenshot({path:`target/0066-browser/rulers-dpr${dpr}.png`});await page.close();
 }
 console.log('PASS actual mounted ruler pixels invalidate on pan/zoom/resize, correct DPR1/2 backing stores');
 }finally{await browser.close();}
})();

// Actual production host frame trace; analyze renderer-main thread durations with sketch-frame-trace.py.
const {chromium}=require('playwright');
const fs=require('node:fs'),path=require('node:path');
const fixture=path.resolve(process.argv[2]||'target/0066-runtime/chain.curvyo');
const output=path.resolve(process.argv[3]||'target/0066-browser');
fs.mkdirSync(output,{recursive:true});
(async()=>{
 const browser=await chromium.launch({headless:true,args:['--no-sandbox','--enable-unsafe-webgpu','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
 try {
 const page=await browser.newPage({viewport:{width:1920,height:1080}});page.on('pageerror',e=>console.log('PAGEERROR',e.stack));
 await page.addInitScript(() => {
   const native = window.requestAnimationFrame.bind(window);
   window.__sketchRafSources = [];
   window.requestAnimationFrame = callback => {
     const source = callback.toString();
     let id = window.__sketchRafSources.indexOf(source);
     if (id < 0) id = window.__sketchRafSources.push(source) - 1;
     const renderer = source.includes('.render(');
     return native(time => {
       performance.mark(`sketch-raf-${id}-start`);
       if (renderer) performance.mark('sketch-render-start');
       callback(time);
       if (renderer) performance.mark('sketch-render-end');
       performance.mark(`sketch-raf-${id}-end`);
     });
   };
   window.addEventListener('pointermove', () => performance.mark('sketch-input'), true);
   new MutationObserver(records => {
     if (records.some(record => record.target instanceof Element && (record.target.closest('[aria-label="Sketch annotations"]') || record.target.getAttribute('aria-label') === 'Sketch annotations'))) performance.mark('sketch-overlay');
   }).observe(document, {subtree:true,childList:true,attributes:true,characterData:true});
 });
 await page.goto(process.env.CURVYO_BROWSER_URL||'http://127.0.0.1:5173/',{waitUntil:'networkidle'});
 const client=await page.context().newCDPSession(page);
 await client.send('Performance.enable',{timeDomain:'threadTicks'});
 console.log('Thread-ticks metrics',JSON.stringify(await client.send('Performance.getMetrics')));
 await page.getByRole('tab',{name:/^File/}).click();
 const chooser=page.waitForEvent('filechooser');await page.getByRole('button',{name:'Open…',exact:true}).click();
 await(await chooser).setFiles(fixture);
 await page.waitForFunction(name=>document.title.includes(name),path.basename(fixture,'.curvyo'));
 await page.getByLabel('Workbench',{exact:true}).selectOption('sketch');
 await page.getByRole('tab',{name:'Sketches',exact:true}).click();
 const active=page.getByLabel('Active sketch');
 console.log('OPENBODY',(await page.locator('body').innerText()).slice(-3000));
 await page.waitForFunction(()=>[...document.querySelectorAll('[aria-label="Active sketch"] option')].some(o=>o.value));
 const options=await active.locator('option').evaluateAll(nodes=>nodes.map(n=>({value:n.value,label:n.textContent})));
 const choice=options.find(o=>o.value);await active.selectOption(choice.value);
 await page.getByLabel('Workbench',{exact:true}).selectOption('sketch');
 await page.getByRole('button',{name:'Select',exact:true}).click();
 const frame=await page.locator('[data-page-frames] > div').first().boundingBox();
 const p=(x,y)=>({x:frame.x+x*frame.width/210,y:frame.y+y*frame.height/297});
 const metric=async()=>Object.fromEntries((await client.send('Performance.getMetrics')).metrics.map(m=>[m.name,m.value]));
 const idle=[];
 for(let i=0;i<8;i++){const before=await metric();await page.evaluate(()=>new Promise(r=>requestAnimationFrame(r)));const after=await metric();idle.push((after.ThreadTime-before.ThreadTime)*1000);}
 const start=p(245,3);await page.mouse.move(start.x,start.y);await page.mouse.down();
 const events=[]; client.on('Tracing.dataCollected',e=>events.push(...e.value));
 await client.send('Tracing.start',{categories:'toplevel,devtools.timeline,blink.user_timing,disabled-by-default-devtools.timeline,disabled-by-default-devtools.timeline.frame',options:'record-until-full',transferMode:'ReportEvents'});
 for(let i=0;i<20;i++) {
   const to=p(244-i*.15,10+i*.2);
   await page.mouse.move(to.x,to.y);
   await new Promise(resolve=>setTimeout(resolve,70));
 }
 await page.mouse.up();
 const done=new Promise(resolve=>client.once('Tracing.tracingComplete',resolve));
 await client.send('Tracing.end');await done;
 fs.writeFileSync(path.join(output,'host-frame-trace.json'),JSON.stringify({traceEvents:events}));
 fs.writeFileSync(path.join(output,'host-raf-sources.json'),JSON.stringify(await page.evaluate(()=>window.__sketchRafSources),null,2));
 console.log('Trace events',events.length);
 await page.screenshot({path:path.join(output,'chain-host.png')});
 await page.getByRole('tab',{name:/^File/}).click();
 const download=page.waitForEvent('download');await page.getByRole('button',{name:'Save',exact:true}).click();
 await(await download).saveAs(path.join(output,'chain-host.curvyo'));
 }finally{await browser.close();}
})();

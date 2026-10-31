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
 await page.getByRole('button',{name:'Workbench',exact:true}).click();
 await page.getByRole('menuitemradio',{name:'Sketch',exact:true}).click();
 await page.getByRole('navigation',{name:'Sketch tools'}).getByRole('button',{name:'Select',exact:true}).click();
 const context=JSON.parse(fs.readFileSync(path.join(path.dirname(fixture),'chain-context.json')));
 const [a,b,c,d,e,f]=context.placement;
 // Zoom around the actual canvas until the entire long chain is reachable.
 await page.mouse.move(900,450);
 await page.keyboard.down("Control");
 await page.mouse.wheel(0,700);
 await page.keyboard.up("Control");
 await page.waitForTimeout(500);
 const frame=await page.locator('[data-page-frames] > div').first().boundingBox();
 const p=(x,y)=>({x:frame.x+(a*x+c*y+e-context.pageOrigin[0])*frame.width/context.pageSize[0],y:frame.y+(b*x+d*y+f-context.pageOrigin[1])*frame.height/context.pageSize[1]});
 const inputs=[p(245,3),...Array.from({length:20},(_,i)=>p(244-i*.15,10+i*.2))];
 for(const endpoint of inputs)if(endpoint.x<260||endpoint.x>1650||endpoint.y<90||endpoint.y>1000)throw Error(`chain input outside canvas ${JSON.stringify(endpoint)}`);
 fs.writeFileSync(path.join(output,'host-input-context.json'),JSON.stringify({context,frame,screenInputs:inputs},null,2));
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

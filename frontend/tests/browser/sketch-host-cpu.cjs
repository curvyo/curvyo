// Informational full-host Chromium thread CPU probe; native Session budget remains separately measured.
const {chromium}=require('playwright');
const fs=require('node:fs');
(async()=>{
 const browser=await chromium.launch({headless:true,args:['--no-sandbox','--enable-unsafe-webgpu','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
 try {
 const page=await browser.newPage({viewport:{width:1920,height:1080}});
 await page.goto(process.env.CURVYO_BROWSER_URL||'http://127.0.0.1:5173/',{waitUntil:'networkidle'});
 const client=await page.context().newCDPSession(page);
 await client.send('Performance.enable',{timeDomain:'threadTicks'});
 console.log('Thread-ticks metrics',JSON.stringify(await client.send('Performance.getMetrics')));
 await page.getByRole('tab',{name:/^File/}).click();
 const chooser=page.waitForEvent('filechooser');await page.getByRole('button',{name:'Open…',exact:true}).click();
 await(await chooser).setFiles('target/0066-runtime/chain.curvyo');
 await page.getByLabel('Workbench',{exact:true}).selectOption('sketch');
 await page.getByRole('tab',{name:'Sketches',exact:true}).click();
 const active=page.getByLabel('Active sketch');
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
 if(process.env.CURVYO_CPU_PROFILE){await client.send('Profiler.enable');await client.send('Profiler.setSamplingInterval',{interval:100});await client.send('Profiler.start');}
 const times=[];
 for(let i=0;i<20;i++){
  const to=p(244-i*.15,10+i*.2),before=await metric();
  await page.mouse.move(to.x,to.y);
  await page.evaluate(()=>new Promise(r=>requestAnimationFrame(r)));
  const after=await metric();times.push((after.ThreadTime-before.ThreadTime)*1000);
 }
 await page.mouse.up();
 if(process.env.CURVYO_CPU_PROFILE){const {profile}=await client.send('Profiler.stop');fs.writeFileSync('target/0066-browser/host-profile.cpuprofile',JSON.stringify(profile));}
 const output={scope:'CDP threadTicks ThreadTime across actual pointer move and first rAF after renderLoop; production bundle, includes host handlers/WASM/draw scheduling; idle measured separately, no subtraction',idle_ms:idle,cold_ms:times[0],warm_max_ms:Math.max(...times.slice(3)),frames_ms:times};
 fs.writeFileSync(process.env.CURVYO_CPU_ARTIFACT||'target/0066-browser/host-cpu-production.json',JSON.stringify(output,null,2));console.log(JSON.stringify(output));
 await page.screenshot({path:'target/0066-browser/chain-host.png'});
 await page.getByRole('tab',{name:/^File/}).click();
 const download=page.waitForEvent('download');await page.getByRole('button',{name:'Save',exact:true}).click();
 await(await download).saveAs('target/0066-browser/chain-host.curvyo');
 }finally{await browser.close();}
})();

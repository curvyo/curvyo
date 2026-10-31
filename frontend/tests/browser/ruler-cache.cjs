// Run against Vite development serving the actual modules, with the exclusive browser slot.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
(async()=>{
 const browser=await chromium.launch({headless:true,args:['--no-sandbox','--enable-unsafe-webgpu','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
 try {
  const page=await browser.newPage();
  await page.goto(process.env.CURVYO_BROWSER_URL||'http://127.0.0.1:1420/',{waitUntil:'networkidle'});
  const results=await page.evaluate(async()=>{
   const {drawRuler}=await import('/src/lib/rulerDraw.ts');
   const {createRulerRaster}=await import('/src/lib/rulerRaster.ts');
   const colours={background:'#eee',tick:'#333',tickMinor:'#aaa',label:'#222',edge:'#888',pointer:'#06f'};
   const data={majors:[0,80,180],minors:[20,40,60],origin:0,labelTicks:[0,80,180],labelTexts:['0','20','40']};
   const results=[];
   for(const horizontal of [true,false])for(const devicePixelRatio of [1,1.5,2])for(const length of [200,321]){
    const original=document.createElement('canvas'),cached=document.createElement('canvas');
    const raster=createRulerRaster(cached,colours);
    const geometry={horizontal,devicePixelRatio,length,thickness:24,pointer:null};
    original.width=Math.round((horizontal?length:24)*devicePixelRatio);original.height=Math.round((horizontal?24:length)*devicePixelRatio);
    raster.rebuild(data,geometry);
    for(const pointer of [null,0,80.25,199.9,null]){
     drawRuler(original.getContext('2d'),data,{...geometry,pointer},colours);raster.paint({...geometry,pointer});
     const a=original.getContext('2d').getImageData(0,0,original.width,original.height).data;
     const b=cached.getContext('2d').getImageData(0,0,cached.width,cached.height).data;
     let maximum=0;for(let i=0;i<a.length;i++)maximum=Math.max(maximum,Math.abs(a[i]-b[i]));
     results.push({horizontal,devicePixelRatio,length,pointer,maximum});
    }
   }
   return results;
  });
  // Transparent text rasterization plus source-over composition rounds twice in 8-bit channels.
  // Allow those two channel units, never a displaced pixel or a geometry-coordinate tolerance.
  for(const result of results)assert(result.maximum<=2,JSON.stringify(result));
  console.log(`PASS ${results.length} cached/uncached ruler pixel comparisons including pointer leave, resize and DPR`);
 }finally{await browser.close();}
})();

// Shared helpers for the 0066 revision 2 browser scripts: the revised Sketch interaction has no
// options bar and no "New"; numeric entry is the on-canvas box (Tab), modes and variants are Alt.
const {chromium}=require('playwright');
const {execFileSync}=require('node:child_process');
const fs=require('node:fs');
const flatten=nodes=>nodes.flatMap(n=>n.objects?flatten(n.objects):[n]);
exports.flatten=flatten;
exports.url=()=>process.env.CURVYO_BROWSER_URL||'http://127.0.0.1:1421/';
exports.launch=()=>chromium.launch({headless:true,args:['--no-sandbox','--enable-unsafe-webgpu','--use-angle=swiftshader','--enable-unsafe-swiftshader']});
exports.menu=async(page,name,choice)=>{await page.getByRole('button',{name,exact:true}).click();await page.getByRole('menuitemradio',{name:choice,exact:true}).click();};
exports.tool=(page,name)=>page.getByRole('navigation',{name:'Sketch tools'}).getByRole('button',{name,exact:true}).click();
exports.rail=(page,name)=>page.getByRole('toolbar',{name:'Constraints'}).locator(`[data-sketch-rail="${name}"]`);
exports.notes=page=>page.getByLabel('Sketch annotations');
exports.pump=page=>page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
/** A fresh page in the Sketch workbench of a new document. */
exports.fresh=async(browser,opts={})=>{
 const page=await browser.newPage({viewport:{width:1440,height:1000},...opts}),errors=[];
 page.on('pageerror',e=>errors.push(e.stack));page.setDefaultTimeout(8000);
 await page.goto(exports.url(),{waitUntil:'networkidle'});
 await exports.menu(page,'Workbench','Sketch');
 return {page,errors};
};
/** Saves through File > Save; returns the document JSON and the first sketch graph. */
exports.save=async(page,root,name)=>{
 fs.mkdirSync(root,{recursive:true});
 await page.getByRole('tab',{name:/^File/}).click();
 const pending=page.waitForEvent('download');await page.getByRole('button',{name:'Save',exact:true}).click();
 const path=`${root}/${name}.curvyo`;await(await pending).saveAs(path);
 const json=JSON.parse(execFileSync('python3',['-I','-c','import zipfile,sys;print(zipfile.ZipFile(sys.argv[1]).read("document.json").decode())',path],{encoding:'utf8'}));
 return {path,json,graph:flatten(json.objects).find(n=>n.shape==='sketch')?.graph};
};
/** Types into the on-canvas entry box: Tab opens it, fields by aria-label, Enter commits. */
exports.typeEntry=async(page,values,{commit=true}={})=>{
 await page.keyboard.press('Tab');
 const box=page.locator('[data-sketch-entry]');await box.waitFor();
 for(const [label,text] of values){const f=box.getByRole('textbox',{name:label,exact:true});await f.fill(text);}
 if(commit)await page.keyboard.press('Enter');
 return box;
};
/** One Alt press per call; the mode label text afterwards. */
exports.alt=async(page,n=1)=>{for(let i=0;i<n;i++){await page.keyboard.press('Alt');await exports.pump(page);}return page.locator('[data-sketch-mode-label]').textContent();};
/** Opens the Sketches tab and its Geometry list (idempotent: the list is a disclosure). */
exports.geometry=async page=>{
 await page.getByRole('tab',{name:'Sketches',exact:true}).click();
 const open=await page.getByLabel('Sketch geometry').evaluate(e=>e.open);
 if(!open)await page.getByText(/^Geometry \(/).click();
 return page.getByLabel('Sketch entities');
};
/** Puts the keyboard focus on the canvas container without a pointer event (as after a panel click). */
exports.focusCanvas=page=>page.locator('div.relative.flex-1.outline-none[tabindex="0"]').first().focus();

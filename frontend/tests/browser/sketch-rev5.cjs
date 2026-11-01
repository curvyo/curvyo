// 0066 customer revision 5: the customer's quadrilateral with a horizontal and a vertical
// distance (AC49, screenshot customer-glitch.png), its text dragged beyond the right point
// (customer-symmetry.png), then Symmetric about the Y axis picked where the dimension line
// crosses it and about the X axis by rail picks (AC52), through real pointer, key and rail events.
// A pixel oracle checks that every dimension-coloured pixel lies on the expected marks.
// REV5_PHASE=before only takes the screenshots (for the build before the fix).
const h=require('./sketch-rev2-support.cjs');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const out='target/0066-rev5';
const phase=process.env.REV5_PHASE||'after';
const check=phase==='after';
(async()=>{
 fs.mkdirSync(out,{recursive:true});
 const browser=await h.launch();
 const {page,errors}=await h.fresh(browser);
 try{
 const settle=()=>page.waitForTimeout(150);
 const shot=name=>page.screenshot({path:`${out}/${phase}-${name}.png`});
 const O=[640,609]; // the sketch origin on screen at 100 %
 const xy=([x,y])=>[O[0]+x,O[1]+y];
 const click=async(p,mods=[])=>{for(const m of mods)await page.keyboard.down(m);const [x,y]=xy(p);await page.mouse.move(x,y);await settle();await page.mouse.click(x,y);await settle();for(const m of mods)await page.keyboard.up(m);};
 const away=async()=>{await page.mouse.move(1100,940);await settle();};
 const texts=()=>h.notes(page).locator('[data-sketch-dimension-text]');
 const centre=async i=>{const b=await texts().filter({hasText:i?'↕':'↔'}).boundingBox();return {x:b.x+b.width/2,y:b.y+b.height/2,w:b.width,h:b.height};};
 // The customer's quadrilateral: top, right, bottom, left, the origin inside.
 const T=[-9,-240],R=[217,102],B=[-10,240],L=[-177,-156];
 await h.tool(page,'Polyline');
 for(const p of [T,R,B,L,T])await click(p);
 await page.keyboard.press('Escape');await settle();await page.keyboard.press('Escape');await settle();
 await click(L);await click(R,['Shift']);
 await h.rail(page,'horizontal-distance').click();await settle();
 await page.keyboard.press('Enter');await settle();
 await page.keyboard.press('Escape');await settle();
 await click(T);await click(B,['Shift']);
 await h.rail(page,'vertical-distance').click();await settle();
 await shot('01-vertical-editing');
 // AC49: the edited dimension's line is whole while its text is hidden.
 const gap=async()=>page.evaluate(async({png,x,y0,y1})=>{
  const img=new Image();img.src='data:image/png;base64,'+png;await img.decode();
  const c=document.createElement('canvas');c.width=img.width;c.height=img.height;
  const g=c.getContext('2d');g.drawImage(img,0,0);
  let missing=[];
  for(let y=y0;y<=y1;y++){let hit=false;for(let dx=-2;dx<=2;dx++){const [r,gr,b]=g.getImageData(x+dx,y,1,1).data;if((b>180&&r<120)||(r<90&&gr<90&&b<90))hit=true;}if(!hit)missing.push(y);}
  // The longest run of rows without the line: a geometry edge crossing it covers one row.
  let run=0,longest=0,prev=-2;for(const y of missing){run=y===prev+1?run+1:1;longest=Math.max(longest,run);prev=y;}
  return {missing,longest};
 },{png:(await page.screenshot()).toString('base64'),x:O[0]+T[0]-24,y0:O[1]+T[1]+12,y1:O[1]+B[1]-12});
 const editingGap=await gap();
 console.log('edited line: rows without the line',JSON.stringify(editingGap));
 await page.keyboard.press('Enter');await settle();
 await page.keyboard.press('Escape');await settle();
 await away();
 await shot('02-axis-distances');
 // Pixel oracle: dimension red (or the selected accent) only on the two dimensions' own marks,
 // the measured points' markers and the origin glyph.
 const oracle=async name=>{
  const [hText,vText]=[await centre(0),await centre(1)];
  const hl=hText.y, vl=vText.x;
  const segs=[
   [xy(L),[xy(L)[0],hl]],[xy(R),[xy(R)[0],hl]],[[xy(L)[0],hl],[xy(R)[0],hl]],
   [xy(T),[vl,xy(T)[1]]],[xy(B),[vl,xy(B)[1]]],[[vl,xy(T)[1]],[vl,xy(B)[1]]],
  ];
  const boxes=[hText,vText].map(t=>[t.x-t.w/2-3,t.y-t.h/2-3,t.x+t.w/2+3,t.y+t.h/2+3]);
  const stray=await page.evaluate(async({png,segs,boxes,origin})=>{
   const img=new Image();img.src='data:image/png;base64,'+png;await img.decode();
   const c=document.createElement('canvas');c.width=img.width;c.height=img.height;
   const g=c.getContext('2d');g.drawImage(img,0,0);
   const d=g.getImageData(0,0,c.width,c.height).data;
   const dist=(px,py,[[ax,ay],[bx,by]])=>{const dx=bx-ax,dy=by-ay,l=dx*dx+dy*dy;const t=l?Math.max(0,Math.min(1,((px-ax)*dx+(py-ay)*dy)/l)):0;return Math.hypot(px-ax-t*dx,py-ay-t*dy);};
   const bad=[];
   for(let y=0;y<c.height;y++)for(let x=250;x<1030;x++){
    const i=(y*c.width+x)*4,r=d[i],gr=d[i+1],b=d[i+2];
    const red=r>140&&r-gr>70&&r-b>60, accent=b>190&&b-r>110&&b-gr>70;
    if(!red&&!accent)continue;
    if(Math.abs(x-origin[0])<=10&&Math.abs(y-origin[1])<=10)continue;
    if(boxes.some(([x0,y0,x1,y1])=>x>=x0&&x<=x1&&y>=y0&&y<=y1))continue;
    // 1px neighbourhood of the 1px lines plus antialiasing; arrowheads are 3px half wide.
    if(segs.some(s=>dist(x+0.5,y+0.5,s)<=4.5))continue;
    bad.push([x,y]);
   }
   return bad;
  },{png:(await page.screenshot()).toString('base64'),segs,boxes,origin:O});
  console.log(name,'stray dimension pixels',stray.length,JSON.stringify(stray.slice(0,8)));
  return stray;
 };
 const strayA=await oracle('axis distances');
 // The customer's second screenshot: the horizontal text dragged far right of the right point.
 const before=await centre(0);
 await page.mouse.move(before.x,before.y);await page.mouse.down();
 await page.mouse.move(before.x+380,before.y,{steps:10});await page.mouse.up();await settle();
 await away();
 await shot('03-text-dragged-beyond');
 const dragged=await centre(0);
 console.log('dragged text centre',dragged.x,'right point',xy(R)[0]);
 const strayB=await oracle('text dragged beyond');
 // AC52: L and R with the Y axis, Shift-picked where the horizontal dimension line crosses it.
 await page.keyboard.press('Escape');await settle();
 await click(L);await click(R,['Shift']);
 const level=dragged.y-O[1];
 await click([0,level],['Shift']);
 await h.rail(page,'symmetric').click();await settle();
 await away();
 await shot('04-symmetric-y-axis');
 // AC52 by rail picks: T and B about the X axis, picked last.
 await page.keyboard.press('Escape');await settle();
 await h.rail(page,'symmetric').click();await settle();
 await click(T);await click(B);await click([-300,0]);
 await away();
 await shot('05-symmetric-x-axis');
 const {graph}=await h.save(page,out,`${phase}-rev5`);
 const symmetric=Object.values(graph.constraints).map(c=>c.relation).filter(r=>r.SymmetricAxis||r.SymmetricPoint||r.Symmetric);
 console.log(JSON.stringify(symmetric));
 if(check){
  assert.ok(editingGap.longest<=1,'the edited dimension line has no gap');
  assert.deepEqual(strayA,[],'no stray dimension pixels');
  assert.deepEqual(strayB,[],'no stray dimension pixels with the text dragged beyond');
  assert.ok(dragged.x<xy(R)[0],'the dragged text stays between the extension lines');
  assert.deepEqual(symmetric.map(r=>Object.keys(r)[0]),['SymmetricAxis','SymmetricAxis'],JSON.stringify(symmetric));
  assert.deepEqual(symmetric.map(r=>r.SymmetricAxis.axis).sort(),['X','Y']);
  assert.deepEqual(errors,[]);
  console.log('rev5 passed');
 }
 }catch(e){await page.screenshot({path:`${out}/${phase}-failure.png`});throw e;}finally{await browser.close();}
})().catch(e=>{console.error(e);process.exit(1);});

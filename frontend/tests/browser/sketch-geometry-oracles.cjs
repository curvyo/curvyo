// Analytic witnesses for saved browser gestures at the documented 100% (96dpi) view.
const assert=require('node:assert/strict');
const near=(a,b)=>assert(Math.abs(a-b)<1e-6,`${a} != ${b}`);
module.exports=(name,g)=>{
 const entries=Object.entries(g.entities),entities=entries.map(([,e])=>e);
 const point=id=>g.entities[id].Point.position;
 const dist=(a,b)=>Math.hypot(a.x-b.x,a.y-b.y);
 const px=25.4/96;
 if(name==='typed-rectangle'){
  const points=entries.filter(([id,e])=>id!==g.origin&&e.Point).map(([,e])=>e.Point.position);
  near(Math.max(...points.map(p=>p.x))-Math.min(...points.map(p=>p.x)),30);
  near(Math.max(...points.map(p=>p.y))-Math.min(...points.map(p=>p.y)),20);
 }
 if(['ellipse','ellipse-three','typed-ellipse'].includes(name)){
  const e=entities.find(e=>e.Ellipse).Ellipse,c=point(e.center),f=point(e.focus);
  const major=Math.hypot(dist(c,f),e.minor_radius);
  near(major,name==='typed-ellipse'?20:50*px);
  near(e.minor_radius,name==='typed-ellipse'?10:name==='ellipse-three'?30*px/Math.sqrt(1-0.4**2):30*px);
  near(c.y,f.y);
 }
 if(name==='circle-three'||name==='arc'){
  const curve=entities.find(e=>e.Circle||e.Arc);const e=curve.Circle||curve.Arc;
  near(e.radius,50*px);
  if(e.start){near(dist(point(e.center),point(e.start)),e.radius);near(dist(point(e.center),point(e.end)),e.radius);near(Math.abs(e.end_angle-e.start_angle),Math.PI);}
 }
 if(name==='slot'){
  const arcs=entities.filter(e=>e.Arc).map(e=>e.Arc),lines=entities.filter(e=>e.Line).map(e=>e.Line);
  assert.equal(arcs.length,2);assert.equal(lines.length,2);
  for(const a of arcs){near(a.radius,30*px);near(dist(point(a.center),point(a.start)),a.radius);near(dist(point(a.center),point(a.end)),a.radius);}
  for(const l of lines){const a=point(l.start),b=point(l.end);near(dist(a,b),100*px);near(a.y,b.y);}
  near(dist(point(arcs[0].center),point(arcs[1].center)),100*px);
 }
};

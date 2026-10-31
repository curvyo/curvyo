// Replays the identical actual segment-body gesture; native verifies every target.
const fs=require('node:fs'),path=require('node:path');
const root=path.resolve(process.argv[2]||'target/0066-runtime-revision');
const {WasmSession}=require(path.resolve(process.argv[3]||`${root}/bindings/curvyo_editor_wasm.js`));
for(const name of ["suffix","tangent"]) {
const context=JSON.parse(fs.readFileSync(`${root}/${name}-context.json`));
const s=WasmSession.open(996n,fs.readFileSync(`${root}/${name}.curvyo`));
try {
 s.set_workbench('sketch');if(!s.enter_sketch(context.owner))throw Error('suffix owner');s.set_sketch_tool('select');
 const zero=s.screen_to_document(0,0),unit=s.screen_to_document(1,1);
 const screen=([x,y])=>{const[a,b,c,d,e,f]=context.placement;const dx=a*x+c*y+e,dy=b*x+d*y+f;const r=[(dx-zero.x)/(unit.x-zero.x),(dy-zero.y)/(unit.y-zero.y)];const check=s.screen_to_document(...r);if(Math.hypot(check.x-dx,check.y-dy)>1e-9)throw Error('mapping');return r;};
 s.pointer_down(...screen(context.start),false,false,false);s.pointer_hover(...screen(context.target),false,false,false);s.pointer_up(...screen(context.target),false,false,false);
 fs.writeFileSync(`${root}/${name}-wasm.curvyo`,s.pack());console.log(`Executed actual ${name} pointer gesture`);
}finally{s.free();}

}

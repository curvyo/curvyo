const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(process.argv[2] || 'target/0066-runtime');
const bindings = path.resolve(process.argv[3] || path.join(root, 'bindings/curvyo_editor_wasm.js'));
const {WasmSession} = require(bindings);
const s = WasmSession.open(721n, fs.readFileSync(`${root}/chain.curvyo`));
s.set_workbench('sketch');
const context = JSON.parse(fs.readFileSync(`${root}/chain-context.json`));
const id = context.owner;
if (!s.enter_sketch(id)) throw new Error('enter');
s.set_sketch_tool('select');
if (s.snapping_enabled()) s.toggle_snapping();
const zero = s.screen_to_document(0,0), unit = s.screen_to_document(1,1);
const screen = ([localX,localY])=>{
 const [a,b,c,d,e,f]=context.placement;
 const x=a*localX+c*localY+e,y=b*localX+d*localY+f;
 const result=[(x-zero.x)/(unit.x-zero.x),(y-zero.y)/(unit.y-zero.y)];
 const check=s.screen_to_document(...result);
 if(Math.hypot(check.x-x,check.y-y)>1e-9) throw new Error('coordinate mapping mismatch');
 return result;
};
const expected = JSON.parse(fs.readFileSync(`${root}/chain-native.json`));
let start = [245,3]; const cpu=[];
for (const [index, record] of expected.entries()) {
 s.pointer_down(...screen(start),false,false,false);
 const before = process.threadCpuUsage();
 s.pointer_hover(...screen(record.target),false,false,false);
 const view=JSON.parse(s.sketch_view_json());
 const elapsed=process.threadCpuUsage(before);
 if(view.error) throw new Error(`frame${index} ${view.error}`);
 cpu.push((elapsed.user+elapsed.system)/1000);
 s.pointer_up(...screen(record.target),false,false,false);
 fs.writeFileSync(`${root}/chain-wasm-${index}.curvyo`, s.pack());
 // Identical next press in both runtimes; expected is unrounded native pose.
 start=record.points[49];
}
console.log(JSON.stringify({runtime:process.version,frames:cpu.length,measured:'pointer+view (no GPU/draw bridge)',maxMs:Math.max(...cpu.slice(3)),cpuMs:cpu}));
s.free();

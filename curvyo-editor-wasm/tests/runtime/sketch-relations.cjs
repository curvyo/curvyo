// Actual production WASM dimension edits; Rust verifies unrounded analytic witnesses.
const fs=require('node:fs'),path=require('node:path');
const root=path.resolve(process.argv[2]||'target/0066-runtime-revision');
const {WasmSession}=require(path.resolve(process.argv[3]||`${root}/bindings/curvyo_editor_wasm.js`));
for(const name of ['axis','left','right','arc-arc']) {
 const context=JSON.parse(fs.readFileSync(`${root}/${name}-context.json`));
 const s=WasmSession.open(992n,fs.readFileSync(`${root}/${name}.curvyo`));
 try {
  s.set_workbench('sketch');if(!s.enter_sketch(context.owner))throw Error(`owner ${name}`);
  s.sketch_dimension_text('size',context.value,false);
  fs.writeFileSync(`${root}/${name}-wasm.curvyo`,s.pack());
  console.log(`Executed ${name} size edit ${context.value}`);
 } finally {s.free();}
}

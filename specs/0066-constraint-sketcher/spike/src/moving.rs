//! Repeated motion checks independent hard distances after each solve.
use serde_json::{json,Value};
use crate::{cases,solve};
#[cfg(unix)] fn cpu()->u128 {let t=rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);t.tv_sec as u128*1_000_000_000+t.tv_nsec as u128}
pub fn run()->Value {
 #[cfg(unix)] let mut timings=vec![];
 let all=cases();let mut req=all.as_array().unwrap().iter().find(|c|c["name"]=="connected_100_entities_drag").unwrap()["request"].clone();
 let ps=req["primitives"].as_array_mut().unwrap();
 ps.retain(|p|p["type"]!="horizontal");
 req["analyze"]=false.into();
 let mut errors=vec![];let mut endpoints=vec![];
 for step in 1..=20 {
  let x=49.0-step as f64*0.3;let y=step as f64*0.15;
  for p in req["primitives"].as_array_mut().unwrap(){if p["id"]=="dragx"{p["value"]=x.into();} if p["id"]=="dragy"{p["value"]=y.into();}}
  #[cfg(unix)] let start=cpu();
  match solve(&req) {Err(e)=>errors.push(e),Ok(r)=>{
   if r["status"]!="converged" {errors.push(format!("step {step}: {}",r["status"]));}
   let ps=r["primitives"].as_array().unwrap();
   let point=|i:usize|{let p=ps.iter().find(|p|p["id"]==format!("p{i}")).unwrap();(p["x"].as_f64().unwrap(),p["y"].as_f64().unwrap())};
   for i in 0..49 {let a=point(i);let b=point(i+1);let d=(b.0-a.0).hypot(b.1-a.1);if !d.is_finite()||(d-1.0).abs()>1e-6{errors.push(format!("step {step} edge {i}: distance {d}"));}}
   let end=point(49);if (end.0-x).hypot(end.1-y)>1e-6{errors.push(format!("step {step}: target miss {end:?} expected {x},{y}"));}
   endpoints.push(json!([end.0,end.1]));req["primitives"]=r["primitives"].clone();
  }}
  #[cfg(unix)] timings.push((cpu()-start) as f64/1e6);
 }
 #[cfg(unix)] {timings.sort_by(f64::total_cmp);eprintln!("moving100 frame median_ms={} max_ms={}",timings[10],timings[19]);}
 json!({"name":"moving_100_entities_20_frames","errors":errors,"endpoints":endpoints})
}

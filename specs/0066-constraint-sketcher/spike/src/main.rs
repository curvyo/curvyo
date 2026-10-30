//! Native corpus output and thread CPU timings.
use curvyo_0066_solver_spike::{cases,corpus_results,solve};
fn now()->u128 {let t=rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);t.tv_sec as u128*1_000_000_000+t.tv_nsec as u128}
fn main(){
    println!("{}",corpus_results());
    let start=now();let moving=curvyo_0066_solver_spike::moving::run();eprintln!("moving100 20frames total_ms={} errors={}",(now()-start) as f64/1e6,moving["errors"]);
    let all=cases();
    for name in ["connected_100_entities_drag","connected_100_entities_failure","p100_rectangles_changed_width"] {
    let case=all.as_array().unwrap().iter().find(|c|c["name"]==name).unwrap();
    for analyze in [false,true] {
        let mut request=case["request"].clone();request["analyze"]=analyze.into();
        let mut samples=vec![];
        for _ in 0..10 {let start=now();let r=solve(&request).unwrap();let ns=now()-start;assert_eq!(r["status"],case["status"]);samples.push(ns as f64/1e6);}
        samples.sort_by(f64::total_cmp);eprintln!("{name} analyze={analyze}: median_ms={} max_ms={}",samples[5],samples[9]);
    }}
}

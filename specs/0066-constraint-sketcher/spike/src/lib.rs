//! Isolated ACS feasibility harness; never imported by the application.
use serde_json::{Value, json};
pub mod moving;
use wasm_bindgen::prelude::*;

pub fn cases() -> Value { serde_json::from_str(include_str!("../corpus.json")).unwrap() }
pub fn solve(request: &Value) -> Result<Value,String> {
    acs::sketch_solve::solve_sketch_json(&request.to_string()).and_then(|s| serde_json::from_str(&s).map_err(|e|e.to_string()))
}
pub fn validate(case:&Value, result:&Value)->Vec<String> {
    let mut errors=vec![];
    if case["status"] != "observe" && case["status"] != result["status"] { errors.push(format!("status {} != {}",result["status"],case["status"])); }
    if case.get("dof").is_some() && case["dof"] != result["dof"] { errors.push(format!("DOF {} != {}",result["dof"],case["dof"])); }
    for check in case["checks"].as_array().unwrap() {
        let item=result["primitives"].as_array().and_then(|ps|ps.iter().find(|p|p["id"]==check[0]));
        let actual=item.and_then(|p|p[check[1].as_str().unwrap()].as_f64());
        if !actual.is_some_and(|v|v.is_finite() && (v-check[2].as_f64().unwrap()).abs()<=1e-6) {errors.push(format!("coordinate {check} actual {actual:?}"));}
    }
    if let Some(ps)=result["primitives"].as_array() {
        for p in ps {for field in ["x","y","radius","radmin","start_angle","end_angle"] {if let Some(v)=p.get(field) {if !v.as_f64().is_some_and(f64::is_finite) {errors.push(format!("nonfinite {} {field}",p["id"]));}}}}
        if let Some(checks)=case["distance_checks"].as_array(){for c in checks {let a=ps.iter().find(|p|p["id"]==c[0]).unwrap();let b=ps.iter().find(|p|p["id"]==c[1]).unwrap();let d=(a["x"].as_f64().unwrap()-b["x"].as_f64().unwrap()).hypot(a["y"].as_f64().unwrap()-b["y"].as_f64().unwrap());if (d-c[2].as_f64().unwrap()).abs()>1e-6{errors.push(format!("bad distance {d}"));}}}
    }
    for field in ["conflicting","redundant"] {
        if let Some(ids)=case[field].as_array() {for id in ids {if !result[field].as_array().is_some_and(|a|a.contains(id)) {errors.push(format!("missing {field} ID {id}: {}",result[field]));}}}
    }
    errors
}
#[wasm_bindgen]
pub fn corpus_results()->String {
    let records:Vec<_>=cases().as_array().unwrap().iter().map(|case|match solve(&case["request"]) {
        Ok(result)=>json!({"name":case["name"],"errors":validate(case,&result),"result":result}),
        Err(error) if case["status"]=="rejected"=>json!({"name":case["name"],"errors":[],"rejection":error}),
        Err(error)=>json!({"name":case["name"],"errors":[error]})
    }).collect();
    let mut records=records; records.push(moving::run());
    serde_json::to_string_pretty(&records).unwrap()
}
#[cfg(test)] mod tests {
    #[test] fn independent_corpus() {
        let results:serde_json::Value=serde_json::from_str(&super::corpus_results()).unwrap();
        let fails:Vec<_>=results.as_array().unwrap().iter().filter(|r|!r["errors"].as_array().unwrap().is_empty()).collect();
        assert!(fails.is_empty(),"{fails:#?}");
    }
}

import json
import sys
from pathlib import Path
root=Path(sys.argv[1] if len(sys.argv)>1 else "target/0066-browser")
es=json.load(open(root/'host-frame-trace.json'))['traceEvents']
inputs=sorted((e for e in es if e['name']=='sketch-input'),key=lambda e:e['ts']);pid,tid=inputs[0]['pid'],inputs[0]['tid'];es=[e for e in es if (e.get('pid'),e.get('tid'))==(pid,tid)]
tasks=sorted((e for e in es if e['name']=='RunTask' and e['ph']=='X'),key=lambda e:e['ts'])
assert len(inputs)==20, f"expected20 actual pointer inputs, got{len(inputs)}"
rows=[]
for index,event in enumerate(inputs):
 start=next(t for t in tasks if t['ts']<=event['ts']<t['ts']+t['dur'])
 overlay=min((e for e in es if e['name']=='sketch-overlay' and e['ts']>event['ts']),key=lambda e:e['ts'])
 render=min((e for e in es if e['name']=='sketch-render-end' and e['ts']>event['ts']),key=lambda e:e['ts'])
 paint=min((e for e in es if e['name']=='Paint' and e['ts']>=max(overlay['ts'],render['ts'])),key=lambda e:e['ts'])
 end=next(t for t in tasks if t['ts']<=paint['ts']<t['ts']+t['dur'])
 included=[t for t in tasks if start['ts']<=t['ts']<=end['ts']]
 assert all('tdur' in task for task in included), 'missing renderer thread CPU durations'
 assert all(a['ts']+a['dur']<=b['ts']+1 for a,b in zip(included,included[1:]))
 rows.append({'frame':index,'cpu_ms':sum(t.get('tdur',0) for t in included)/1000,'tasks':len(included),'input_task_cpu_ms':start.get('tdur',0)/1000,'completed_frame_task_cpu_ms':end.get('tdur',0)/1000,'missing_cpu_wall_us':sum(t['dur'] for t in included if 'tdur' not in t),'elapsed_ms':(end['ts']+end['dur']-start['ts'])/1000,'start_ts':start['ts'],'end_ts':end['ts']+end['dur']})
out={'method':'All nonoverlapping renderer-main RunTask thread durations from pointer input task through first Paint task after both updated canvas render and React overlay mutation; no idle subtraction or protocol tasks excluded','cold_ms':rows[0]['cpu_ms'],'warm_max_ms':max(r['cpu_ms'] for r in rows[3:]),'frames':rows}
json.dump(out,open(root/'host-frame-cpu.json','w'),indent=2);print(json.dumps(out,indent=2))

assert out['warm_max_ms'] < 16.0, f"warmed complete-frame CPU exceeds16ms: {out['warm_max_ms']}"

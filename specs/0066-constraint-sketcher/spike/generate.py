import json, math
from pathlib import Path
cases=[]
def p(i,x,y,f=False): return dict(type='point',id=i,x=x,y=y,fixed=f)
def l(i,a,b): return dict(type='line',id=i,p1_id=a,p2_id=b)
def k(t,i,**kw): return dict(type=t,id=i,**kw)
def add(name,ps,dof=None,checks=[],status='converged',**extra):
 c=dict(name=name,request=dict(version=1,primitives=ps),status=status,checks=checks,**extra)
 if dof is not None:c['dof']=dof
 cases.append(c)
def ck(i,field,n): return [i,field,n]
add('free_point',[p('p',1,2)],2)
add('fixed_point',[p('p',1,2,True)],0,[ck('p','x',1)])
add('free_circle',[p('c',0,0),k('circle','c1',c_id='c',radius=3)],3)
add('free_ellipse',[p('c',0,0),p('f',4,0),k('ellipse','e',c_id='c',focus1_id='f',radmin=3)],5)
arc=[p('c',0,0),p('s',3,0),p('e',0,3),k('arc','a',c_id='c',start_id='s',end_id='e',radius=3,start_angle=0,end_angle=math.pi/2)]
add('free_arc',arc,5)
base=[p('a',0,0,True),p('b',8,2),l('l','a','b')]
full=base+[k('horizontal','h',line='l'),k('length','len',curve='l',value=10)]
add('typed_length_horizontal',full,0,[ck('b','x',10),ck('b','y',0)])
add('typed_length_vertical',base+[k('vertical','v',line='l'),k('length','len',curve='l',value=10)],0,[ck('b','x',0),ck('b','y',10)])
add('coincident',[p('a',2,3,True),p('b',0,0),k('coincident','co',a='a',b='b')],0,[ck('b','x',2),ck('b','y',3)])
add('distance_direction',base+[k('distance','d',a='a',b='b',value=10),k('direction','dir',line='l',value=math.pi/6)],0,[ck('b','x',math.sqrt(75)),ck('b','y',5)])
for t,angle,xy in [('parallel',0,(10,0)),('perpendicular',math.pi/2,(0,10)),('angle',math.pi/4,(math.sqrt(50),math.sqrt(50)))]:
 ps=base+[p('r',10,0,True),l('ref','a','r'),k('length','len',curve='l',value=10)]
 ps += [k(t,'rel',a='ref',b='l',**(dict(value=angle,reverseA=False,reverseB=False) if t=='angle' else {}))]
 add(t,ps,0,[ck('b','x',xy[0]),ck('b','y',xy[1])])
for t,v in [('radius',5),('diameter',10)]:
 add(t,[p('c',0,0,True),k('circle','c1',c_id='c',radius=3),k(t,'dim',curve='c1',value=v)],0,[ck('c1','radius',5)])
add('equal_radius',[p('a',0,0,True),p('b',10,0,True),k('circle','c1',c_id='a',radius=3,fixed=True),k('circle','c2',c_id='b',radius=1),k('equal','eq',a='c1',b='c2')],0,[ck('c2','radius',3)])
add('midpoint',[p('a',0,0,True),p('b',10,0,True),p('m',3,2),l('l','a','b'),k('midpoint','mid',entities=['m','l'])],0,[ck('m','x',5),ck('m','y',0)])
add('symmetric',[p('a',0,0,True),p('b',0,10,True),p('s',3,2,True),p('m',-2,1),l('axis','a','b'),k('mirror','sym',source='s',image='m',axis='axis')],0,[ck('m','x',-3),ck('m','y',2)])
add('duplicate_ids',full+[k('length','len_dup',curve='l',value=10)],0,[ck('b','x',10)],redundant=['len_dup'])
add('conflicting_ids',full+[k('length','len_conflict',curve='l',value=12)],status='failed',conflicting=['len','len_conflict'])
add('hard_constraints_soft_drag',full+[k('x','gx',point='b',value=30,temporary=True),k('y','gy',point='b',value=30,temporary=True)],0,[ck('b','x',10),ck('b','y',0)])
add('sliding_soft_drag',[p('a',0,0,True),p('b',10,0,True),p('m',3,0),l('l','a','b'),k('on','on',point='m',curve='l'),k('x','gx',point='m',value=6,temporary=True),k('y','gy',point='m',value=3,temporary=True)],1,[ck('m','x',6),ck('m','y',0)])
line=[p('a',-10,0,True),p('b',10,0,True),l('l','a','b')]
add('line_circle_tangent',line+[p('c',0,4),k('circle','c1',c_id='c',radius=3,fixed=True),k('x','cx',point='c',value=0),k('tangent','tan',a='l',b='c1')],0,[ck('c','y',3)])
add('line_ellipse_tangent',line+[p('c',0,4),p('f',4,4),k('ellipse','e',c_id='c',focus1_id='f',radmin=3,fixed=True),k('x','cx',point='c',value=0),k('x','fx',point='f',value=4),k('horizontal','orient',a='c',b='f'),k('tangent','tan',a='l',b='e')],0,[ck('c','y',3),ck('f','y',3)])
# Arc's bottom endpoint is shared with the fixed horizontal segment.
add('line_arc_tangent',[p('c',0,3),p('s',0,0,True),p('e',3,3),p('a',-10,0,True),l('l','a','s'),k('arc','arc',c_id='c',start_id='s',end_id='e',radius=3,start_angle=-math.pi/2,end_angle=0),k('radius','r',curve='arc',value=3),k('angle','sweep',arc='arc',value=math.pi/2),k('tangent','tan',a='l',b='arc')],0,[ck('c','x',0),ck('c','y',3),ck('e','x',3),ck('e','y',3)])
add('zero_length_nonzero_dimension',[p('a',0,0,True),p('b',0,0),l('l','a','b'),k('length','len',curve='l',value=10)],1,[],distance_checks=[['a','b',10]])
add('near_circular_ellipse',[p('c',0,0),p('f',1e-10,0),k('ellipse','e',c_id='c',focus1_id='f',radmin=3)],5)
add('tiny_radius',[p('c',0,0,True),k('circle','c1',c_id='c',radius=2e-5),k('radius','r',curve='c1',value=1e-5)],0,[ck('c1','radius',1e-5)])
# Connected chain: 50 points + 49 lines + one circle = exactly 100 geometry entities.
chain=[p(f'p{i}',float(i),0,i==0) for i in range(50)]
for i in range(49):chain += [l(f'l{i}',f'p{i}',f'p{i+1}'),k('length',f'd{i}',curve=f'l{i}',value=1),k('horizontal',f'h{i}',line=f'l{i}')]
chain += [k('circle','circle',c_id='p0',radius=2),k('radius','rad',curve='circle',value=2),k('x','dragx',point='p49',value=55,temporary=True),k('y','dragy',point='p49',value=8,temporary=True)]
add('connected_100_entities_drag',chain,0,[ck('p49','x',49),ck('p49','y',0)])
# Directed angles across the -pi/pi seam; known unique endpoint.
for angle in [math.pi-1e-5,math.pi+1e-5,-math.pi+1e-5]:
 add('angle_wrap_'+str(angle),[p('a',0,0,True),p('b',-9,0.1),l('l','a','b'),k('length','len',curve='l',value=10),k('direction','dir',line='l',value=angle)],0,[ck('b','x',10*math.cos(angle)),ck('b','y',10*math.sin(angle))])
add('conflicting_radii',[p('c',0,0,True),k('circle','c1',c_id='c',radius=3),k('radius','r1',curve='c1',value=3),k('radius','r2',curve='c1',value=4)],status='failed',conflicting=['r1','r2'])
# Circle-circle external tangency and both circle centers pinned in one axis.
add('circle_circle_tangent',[p('a',0,0,True),p('b',7,0),k('circle','c1',c_id='a',radius=3,fixed=True),k('circle','c2',c_id='b',radius=2,fixed=True),k('y','by',point='b',value=0),k('tangent','t',a='c1',b='c2')],0,[ck('b','x',5),ck('b','y',0)])
# Connected 100-entity chain with a contradictory final edge length.
add('connected_100_entities_failure',chain+[k('length','bad',curve='l48',value=2)],status='failed',conflicting=['d48','bad'])
# Exact ellipse combinations: rejection is recorded as a capability gap, not support.
for other in ['circle','ellipse']:
 ps=[p('c',0,0,True),p('f',4,0,True),k('ellipse','e',c_id='c',focus1_id='f',radmin=3,fixed=True),p('c2',8,0),p('f2',12,0)]
 ps += [k(other,'other',c_id='c2',**(dict(radius=3) if other=='circle' else dict(focus1_id='f2',radmin=3))),k('tangent','tan',a='e',b='other')]
 add('unsupported_ellipse_'+other,ps,status='rejected')
add('missing_reference',[p('a',0,0),l('bad','a','missing')],status='rejected')
add('duplicate_id',[p('a',0,0),p('a',1,1)],status='rejected')
# Radius edit of a line/arc fillet, same topology and IDs, known tangent endpoints.
for radius in [2,4,0.01]:
 ps=[p('c',radius,radius,True),p('s',0,radius),p('e',radius,0),p('v',0,10,True),p('h',10,0,True),l('lv','v','s'),l('lh','e','h'),k('arc','a',c_id='c',start_id='s',end_id='e',radius=radius,start_angle=math.pi,end_angle=1.5*math.pi),k('radius','r',curve='a',value=radius),k('tangent','tv',a='lv',b='a'),k('tangent','th',a='lh',b='a')]
 add('fillet_radius_'+str(radius),ps,0,[ck('s','x',0),ck('s','y',radius),ck('e','x',radius),ck('e','y',0)])
# Tester's P100: 80 line segments +20 circles, with100 backing points explicit.
p100=[]
for i in range(20):
 x=i*30
 p100 += [p(f'b{i}',x,0,True),p(f'r{i}',x+12,0),p(f't{i}',x+12,10),p(f'u{i}',x,10),p(f'c{i}',x+5,5,True)]
 for name,a,b in [('bottom','b','r'),('right','r','t'),('top','u','t'),('left','b','u')]:
  id=f'{name}{i}';p100 += [l(id,f'{a}{i}',f'{b}{i}'),k('horizontal' if name in ['bottom','top'] else 'vertical','axis'+id,line=id)]
 p100 += [k('length',f'height{i}',curve=f'left{i}',value=10),k('circle',f'circle{i}',c_id=f'c{i}',radius=2,fixed=True)]
add('p100_rectangles_free_widths',p100,20)
connected=p100+[k('equal',f'equal{i}',a='bottom0',b=f'bottom{i}') for i in range(1,20)]
add('p100_rectangles_equal_width',connected,1)
add('p100_rectangles_changed_width',connected+[k('length','width',curve='bottom0',value=15)],0,[ck(f'r{i}','x',i*30+15) for i in range(20)])
add('fixed_geometry_contradiction',[p('a',0,0,True),p('b',10,1,True),l('l','a','b'),k('horizontal','bad',line='l')],status='failed',conflicting=['bad'])
# Exact common-contact construction; center line is NOT the normal here.
q=(5/math.sqrt(2),3/math.sqrt(2)); n=(1/5/math.sqrt(2),1/3/math.sqrt(2)); norm=math.hypot(*n); n=(n[0]/norm,n[1]/norm)
for other in ['circle','ellipse']:
 center=(q[0]+2*n[0],q[1]+2*n[1]) if other=='circle' else (2*q[0],2*q[1])
 ps=[p('c',0,0,True),p('f',4,0,True),k('ellipse','e',c_id='c',focus1_id='f',radmin=3,fixed=True),p('c2',center[0]+0.2,center[1]),k('y','center_y',point='c2',value=center[1])]
 if other=='circle':ps += [k('circle','other',c_id='c2',radius=2,fixed=True)]
 else:ps += [p('f2',center[0]+4.2,center[1]),k('ellipse','other',c_id='c2',focus1_id='f2',radmin=3,fixed=True),k('horizontal','orient2',a='c2',b='f2'),k('distance','focus_distance',a='c2',b='f2',value=4)]
 ps += [p('contact',q[0]+0.02,q[1]+0.02),p('tip',q[0]-n[1],q[1]+n[0]),l('tangent','contact','tip'),k('on','contact1',point='contact',curve='e'),k('on','contact2',point='contact',curve='other'),k('tangent','tangent1',a='tangent',b='e'),k('tangent','tangent2',a='tangent',b='other'),k('length','gauge',curve='tangent',value=1)]
 add('adapter_ellipse_'+other,ps,0,[ck('c2','x',center[0]),ck('contact','x',q[0]),ck('contact','y',q[1])])
# Rotated copies preserve off-axis contact geometry; constrain the rotated centre Y.
import copy
for basecase in list(cases):
 if not basecase['name'].startswith('adapter_ellipse_'):continue
 c=copy.deepcopy(basecase);c['name']+='_'+'rotated';ang=0.63;co=math.cos(ang);si=math.sin(ang)
 def rot(x,y):return (co*x-si*y,si*x+co*y)
 ps=c['request']['primitives'];old={p['id']:(p['x'],p['y']) for p in ps if p['type']=='point'}
 for item in ps:
  if item['type']=='point':item['x'],item['y']=rot(item['x'],item['y'])
  if item['id']=='orient2':item.update(type='direction',value=ang)
 cx=next(check[2] for check in basecase['checks'] if check[:2]==['c2','x']);cy=next(item['value'] for item in ps if item['id']=='center_y');correct=rot(cx,cy)
 for item in ps:
  if item['id']=='center_y':item['value']=correct[1]
 contact=rot(q[0],q[1]);c['checks']=[ck('c2','x',correct[0]),ck('contact','x',contact[0]),ck('contact','y',contact[1])];cases.append(c)
Path('target/0066-solver-spike/corpus.json').write_text(json.dumps(cases,indent=2)+'\n')

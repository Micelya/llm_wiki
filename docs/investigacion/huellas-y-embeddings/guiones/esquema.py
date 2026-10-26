import json,sys,re
def mask(v):
    if isinstance(v,str): return 'S%d'%len(v)
    if isinstance(v,(int,float,bool)) or v is None: return type(v).__name__
    if isinstance(v,list): return [mask(v[0])]+(['...x%d'%len(v)] if len(v)>1 else []) if v else []
    if isinstance(v,dict): return {k:mask(x) for k,x in v.items()}
for f in sys.argv[1:]:
    d=json.load(open(f)); print('##',f); print(json.dumps(mask(d))[:1500])

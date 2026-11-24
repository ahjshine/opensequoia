import urllib.request, hashlib, json
from html.parser import HTMLParser
from pathlib import Path
class P(HTMLParser):
 def __init__(self): super().__init__(); self.parts=[]; self.skip=0
 def handle_starttag(self,t,a):
  if t in ('script','style'): self.skip+=1
 def handle_endtag(self,t):
  if t in ('script','style'): self.skip=max(0,self.skip-1)
  if t in ('p','tr','li','h1','h2','h3','pre','div'): self.parts.append('\n')
 def handle_data(self,d):
  if not self.skip:self.parts.append(d)
items=[["profile","https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-createappcontainerprofile"],["derive","https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-deriveappcontainersidfromappcontainername"],["delete","https://learn.microsoft.com/en-us/windows/win32/api/userenv/nf-userenv-deleteappcontainerprofile"],["security-capabilities","https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-security_capabilities"],["attributes","https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute"],["isolation","https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation"],["token","https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-gettokeninformation"],["acl","https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-setnamedsecurityinfow"],["loopback","https://learn.microsoft.com/en-us/windows/win32/winsock/using-winsock-in-an-appcontainer"],["mitigation","https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocessmitigationpolicy"]]
records=[]
for name,url in items:
 try:
  raw=urllib.request.urlopen(url,timeout=25).read(); p=P();p.feed(raw.decode('utf-8'))
  text='\n'.join(' '.join(x.split()) for x in ''.join(p.parts).splitlines() if x.strip())
  out=Path('evidence/verification/sandbox3-doc-'+name+'.txt'); out.write_text(text,encoding='utf-8')
  records.append({'name':name,'url':url,'sha256_html':hashlib.sha256(raw).hexdigest(),'sha256_text':hashlib.sha256(out.read_bytes()).hexdigest(),'bytes':len(raw),'path':str(out),'result':'ok'})
 except Exception as e: records.append({'name':name,'url':url,'result':repr(e)})
Path('evidence/verification/sandbox3-docs.json').write_text(json.dumps(records,indent=2)+'\n',encoding='utf-8')
print(json.dumps(records,indent=2))

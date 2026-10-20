// Opt-in development diagnostic; never imported by the application.
import { chromium } from '@playwright/test';
import { spawn, execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
const root=resolve(import.meta.dirname,'../..');
const limited=process.argv.includes('--limited');
const limit=process.argv.includes('--one')?1:2;
const proc=spawn(resolve(root,'target/release/orbit.exe'),['--isolated','--data-dir',resolve(root,`artifacts/resource-experiment-${Date.now()}`),'--running','--all-widgets','--quit-after','120'],{windowsHide:true,env:{...process.env,WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:`--remote-debugging-port=19341 ${limited?`--renderer-process-limit=${limit}`:''} ${process.argv.includes('--lite')?'--js-flags=--lite-mode':''}`},stdio:'ignore'});
let browser;
const invoke=(page,name,args={})=>page.evaluate(({name,args})=>window.__TAURI_INTERNALS__.invoke(name,args),{name,args});
const python=`import psutil,time,json,sys
p=psutil.Process(int(sys.argv[1]))
def samples():
    result={}
    for x in [p,*p.children(recursive=True)]:
        try:
            cpu=x.cpu_times();result[x.pid]=(cpu.user+cpu.system,x.memory_info().private,x.name())
        except psutil.NoSuchProcess:pass
    return result
a=samples();t=time.perf_counter();time.sleep(4);b=samples()
print(json.dumps(dict(privateMiB=sum(x[1] for x in b.values())/1048576,cpuPercentOneCore=100*sum(max(0,x[0]-a.get(i,(x[0],))[0]) for i,x in b.items())/(time.perf_counter()-t),processes=len(b))))`;
try{
  for(let i=0;i<100;i++){try{if((await fetch('http://127.0.0.1:19341/json/version')).ok)break;}catch{}await delay(200);}
  browser=await chromium.connectOverCDP('http://127.0.0.1:19341');
  for(let i=0;i<100;i++){if(browser.contexts().flatMap(c=>c.pages()).filter(p=>p.url().includes('widget')).length===3)break;await delay(100);}
  const pages=browser.contexts().flatMap(c=>c.pages());const main=pages.find(p=>!p.url().includes('widget'));
  await main.getByRole('heading',{name:'One thing at a time.'}).waitFor();await delay(2000);
  function measure(name){console.log(JSON.stringify({limited,variant:name,...JSON.parse(execFileSync('python',['-c',python,String(proc.pid)],{encoding:'utf8'}))}));}
  measure('baseline');
  for(const page of pages)await page.addStyleTag({content:'.dial-progress{filter:none!important}.orbital-satellite{animation:none!important}'});
  await delay(500);measure('no-svg-filter-or-orbit');
  for(const page of pages)await page.evaluate(()=>{const c=document.querySelector('.dial-progress');if(c)c.replaceWith(c.cloneNode(true));});
  await delay(500);measure('static-svg-progress');
  const s=await invoke(main,'orbit_command',{command:{type:'snapshot'}});
  await invoke(main,'orbit_command',{command:{type:'save_settings',settings:{...s.value.settings,reduced_motion:true}}});
  await delay(1000);measure('reduced-motion');
  await invoke(main,'window_command',{action:'discard_and_quit'});await delay(1000);
}finally{await browser?.close().catch(()=>{});if(proc.exitCode===null)proc.kill();}

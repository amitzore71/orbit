// Real WebView2 + Rust integration. Uses only an isolated test database.
import { chromium } from '@playwright/test';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
const root = resolve(import.meta.dirname, '../..');
const exe = resolve(process.argv[2] || `${root}/target/release/orbit.exe`);
const data = resolve(root, `artifacts/native-ui-${Date.now()}`);
const cycles = Number(process.env.ORBIT_WIDGET_CYCLES || 10);
await mkdir(data, {recursive: true});
const port = 19340;
const maxSeconds = Math.max(180, Math.ceil(cycles * .6) + 120);
const proc = spawn(exe, ['--isolated', '--data-dir', data, '--startup-probe', '--quit-after', String(maxSeconds)], {windowsHide: true, env: {...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`}, stdio: ['ignore', 'pipe', 'pipe']});
let logs = '';
proc.stdout.on('data', x => logs += x); proc.stderr.on('data', x => logs += x);
let browser;
const invoke = async (page, name, args = {}) => page.evaluate(({name, args}) => window.__TAURI_INTERNALS__.invoke(name, args), {name, args});
try {
  for (let i = 0; i < 150; i++) {
    if (proc.exitCode !== null) throw new Error(`Orbit exited: ${proc.exitCode}\n${logs}`);
    try {const r = await fetch(`http://127.0.0.1:${port}/json/version`); if (r.ok) break;} catch {}
    await delay(200);
  }
  browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  let page;
  for (let i = 0; i < 100; i++) {page = browser.contexts().flatMap(c => c.pages()).find(p => !p.url().includes('/widget/')); if (page && page.url() !== 'about:blank') break; await delay(100);}
  if (!page) throw new Error('No native main WebView was created.');
  const errors = []; page.on('pageerror', e => errors.push(e.message));
  page.on('console', m => {if (m.type() === 'error') logs += `\n${m.text()}`;});
  await page.getByRole('heading', {name: 'One thing at a time.'}).waitFor({timeout: 25000});
  await page.getByRole('textbox', {name: 'Task title'}).fill('Native bridge integration 日本語');
  await page.getByRole('button', {name: 'Add task', exact: true}).click();
  await page.evaluate(() => {
    window.__ORBIT_FEEDBACK_MS__ = null;
    document.addEventListener('click', function start(e) {
      const button=e.target.closest('button');
      if (!button?.getAttribute('aria-label')?.startsWith('Start focus on')) return;
      document.removeEventListener('click',start,true);
      const began=performance.now();
      const observer=new MutationObserver(()=>{if([...document.querySelectorAll('button')].some(b=>b.textContent.includes('Pause session'))){observer.disconnect();requestAnimationFrame(()=>{window.__ORBIT_FEEDBACK_MS__=performance.now()-began;});}});
      observer.observe(document.body,{subtree:true,childList:true,characterData:true});
    },true);
  });
  await page.getByRole('button', {name: 'Start focus on Native bridge integration 日本語'}).click();
  await page.getByRole('button', {name: /Pause session/}).waitFor();
  await page.waitForFunction(()=>typeof window.__ORBIT_FEEDBACK_MS__==='number');
  const feedbackMs=await page.evaluate(()=>window.__ORBIT_FEEDBACK_MS__);
  const frameIntervals=await page.evaluate(()=>new Promise(resolve=>{
    const deltas=[];let last=performance.now();
    function frame(now){if(deltas.length)deltas.push(now-last);else deltas.push(0);last=now;if(deltas.length<121)requestAnimationFrame(frame);else resolve(deltas.slice(1));}
    requestAnimationFrame(frame);
  }));
  const sortedFrames=frameIntervals.toSorted((a,b)=>a-b);
  const performanceResult={executableSha256:createHash('sha256').update(await readFile(exe)).digest('hex').toUpperCase(),clickToFeedbackMs:feedbackMs,frameIntervalP95Ms:sortedFrames[Math.ceil(sortedFrames.length*.95)-1],frameSamples:sortedFrames.length,scope:'Real WebView2: DOM click through rendered timer feedback; rAF intervals while focusing. One click sample; excludes physical display latency.'};
  await writeFile(`${root}/artifacts/native-ui-performance.json`,JSON.stringify(performanceResult,null,2));
  await page.getByRole('button',{name:'Insights',exact:true}).click();
  await delay(2200);
  await page.getByRole('button',{name:'Focus',exact:true}).first().click();
  await page.locator('.focus-workspace').waitFor();
  // Wait for the asynchronous snapshot commit, rather than sampling an arbitrary
  // 400 ms during navigation while other isolated WebViews may be starting.
  const navigationDeadline=Date.now()+2000;
  while(true){
    const display=await page.locator('.time-display').innerText();
    const [displayMin,displaySec]=display.replace(/\s/g,'').split(':').map(Number);
    const afterNavigation=await invoke(page,'orbit_command',{command:{type:'snapshot'}});
    if(Math.abs((displayMin*60+displaySec)*1000-afterNavigation.value.timer.remaining_ms)<=1100)break;
    if(Date.now()>navigationDeadline)throw new Error('Returning to Focus displayed stale timer state.');
    await delay(50);
  }
  await page.getByRole('button', {name: /Pause session/}).click();
  const reply = await invoke(page, 'orbit_command', {command: {type: 'snapshot'}});
  if (reply.value.timer.running || reply.value.tasks.length !== 1 || !reply.value.timer.bound_task) throw new Error('Real timer/task state did not match the UI.');
  const saved = reply.value;
  const widgetCommand = async (index, enabled) => invoke(page, 'orbit_command', {command: {type: 'widget', index, preferences: {...saved.widgets[index], enabled}}});
  for (let index = 0; index < 3; index++) await widgetCommand(index, true);
  for (let i = 0; i < 100; i++) {if (browser.contexts().flatMap(c => c.pages()).length >= 4) break; await delay(100);}
  let widgetPages = [];
  for (let i=0;i<100;i++) {widgetPages=browser.contexts().flatMap(c=>c.pages()).filter(p=>p.url().includes('widget'));if(widgetPages.length===3)break;await delay(100);}
  if (widgetPages.length !== 3) throw new Error(`Expected all three widgets, found ${widgetPages.length}. Pages: ${browser.contexts().flatMap(c => c.pages()).map(p=>p.url()).join(', ')}`);
  await delay(700);
  await page.screenshot({path: `${root}/artifacts/redesign/native-focus.png`});
  for (let index = 0; index < widgetPages.length; index++) {
    const w = widgetPages[index];
    await w.getByRole('button', {name: 'Close widget'}).waitFor();
    await w.screenshot({path: `${root}/artifacts/redesign/native-widget-${index}.png`});
  }
  // The task widget and main window read the same task IDs and timer state.
  const taskWidget = widgetPages.find(p => p.url().includes('kind=1'));
  const widgetSnapshot = await invoke(taskWidget, 'orbit_command', {command: {type: 'snapshot'}});
  if (widgetSnapshot.value.timer.bound_task !== saved.timer.bound_task) throw new Error('Widget timer state diverged.');
  // Verify the native host rejects file-path injection through the generic web command.
  let denied = false;
  try {await invoke(page, 'orbit_command', {command: {type: 'backup', path: `${data}/forbidden.db`}});} catch {denied = true;}
  if (!denied) throw new Error('Generic IPC exposed file operations.');
  await invoke(page, 'window_command', {action: 'hide'});
  await delay(200);
  if (proc.exitCode !== null) throw new Error('Hiding the main window terminated the app.');
  await invoke(taskWidget, 'window_command', {action: 'show'});
  // Repeated widget creation/destruction exercises native subclass and callback cleanup.
  for (let cycle = 0; cycle < cycles; cycle++) {await widgetCommand(0, false); await delay(200); await widgetCommand(0, true); await delay(200);}
  await page.getByRole('button', {name: 'Settings', exact: true}).click();
  await page.getByRole('spinbutton', {name: 'Focus minutes', exact: true}).fill('31');
  await page.getByRole('button', {name: 'Save changes', exact: true}).click();
  await page.getByText('All changes saved', {exact: true}).waitFor();
  await page.screenshot({path: `${root}/artifacts/redesign/native-settings.png`});
  const afterSettings=await invoke(page,'orbit_command',{command:{type:'snapshot'}});
  await invoke(page,'orbit_command',{command:{type:'save_settings',settings:{...afterSettings.value.settings,appearance:1,reduced_motion:true,mica:true}}});
  await page.locator('.orbit-app[data-theme="light"][data-reduced-motion="true"]').waitFor();
  await page.getByRole('button',{name:'Focus',exact:true}).first().click();
  await page.locator('.focus-workspace').waitFor();
  await delay(300);
  await page.screenshot({path:`${root}/artifacts/redesign/native-light-reduced.png`});
  await invoke(page,'orbit_command',{command:{type:'save_settings',settings:afterSettings.value.settings}});
  await page.getByRole('button',{name:'Settings',exact:true}).click();
  const discard=page.getByRole('button',{name:'Discard',exact:true});
  if(await discard.isEnabled())await discard.click();
  await page.getByText('All changes saved',{exact:true}).waitFor();
  await delay(200);
  await invoke(page, 'window_command', {action: 'quit'});
  for (let i = 0; i < 100 && proc.exitCode === null; i++) await delay(100);
  if (proc.exitCode !== 0) throw new Error(`Normal native quit failed: ${proc.exitCode}`);
  if (errors.length) throw new Error(errors.join('\n'));
  await writeFile(`${root}/artifacts/native-ui-check.json`, JSON.stringify({dataDirectory: data, realRustBridge: true, realWebView2: true, unicodeTasks: true, taskBinding: true, allWidgets: true, widgetCycles: cycles, closeToTray: true, settingsSave: true, pathInjectionDenied: true, pageErrors: errors, logs}, null, 2));
  console.log('Native WebView2 + Rust integration passed.');
} catch (e) {
  console.error(logs);
  if (browser) {for (const [i, page] of browser.contexts().flatMap(c => c.pages()).entries()) await page.screenshot({path: `${root}/artifacts/redesign/native-failure-${i}.png`}).catch(() => {});}
  throw e;
} finally {await browser?.close().catch(() => {}); if (proc.exitCode === null) proc.kill();}

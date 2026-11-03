// Runs the built component against a mock Horizon host with `jco transpile`.
// Checks the parts that the sandboxed API really allows: registration,
// English settings page, saved settings (including a simulated restart),
// F5 / Shift+F5 gating, the honest status message, and the command feedback.
// It also asserts that NO camera function is ever called: none exists.
import { readFileSync, readdirSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const out = resolve(root, '.arena/mock');
execFileSync(
  resolve(root, 'node_modules/.bin/jco'),
  ['transpile', resolve(root, 'mod.wasm'), '-o', out,
   '--no-typescript', '--no-wasi-shim', '-I', 'sync', '-q'],
  { cwd: root, stdio: 'inherit' },
);
const { instantiate } = await import(pathToFileURL(resolve(out, 'mod.js')).href);
const modules = new Map(
  readdirSync(out)
    .filter(name => name.endsWith('.wasm'))
    .map(name => [name, new WebAssembly.Module(readFileSync(resolve(out, name)))]),
);
const getCoreModule = name => {
  const m = modules.get(name);
  if (!m) throw new Error(`unknown core module ${name}`);
  return m;
};

const dec = new TextDecoder();
const enc = new TextEncoder();
const store = new Map();
const calls = {
  registerKey: [], poll: 0, setSettings: [], set: [],
  log: [], chat: [], hud: [], hudClear: 0, camera: [],
};
let pending = [];
let playerYaw = 0;
const player = () => ({
  x: 0.5, y: 64, z: 0.5,
  yaw: playerYaw, pitch: 0,
  health: 20, food: 20,
  dimension: 'minecraft:overworld',
});

const specificImports = {
  'horizon:mods/log': {
    write: (level, message) => { calls.log.push({ level, message }); },
  },
  'horizon:mods/game': {
    getPlayer: () => player(),
    getClock: () => undefined,
    raycast: () => undefined,
  },
  'horizon:mods/chat': {
    show: text => { calls.chat.push(text); },
  },
  'horizon:mods/hud': {
    setPanel: (corner, lines) => { calls.hud.push({ corner, lines }); },
    clear: () => { calls.hudClear++; },
  },
  'horizon:mods/storage': {
    get: k => store.get(k),
    set: (k, v) => {
      const bytes = new Uint8Array(v);
      store.set(k, bytes);
      calls.set.push([k, dec.decode(bytes)]);
    },
    delete: k => store.delete(k),
  },
  'horizon:mods/input': {
    registerKey: (name, label, key) => { calls.registerKey.push({ name, label, key }); },
    poll: () => { calls.poll++; const value = pending; pending = []; return value; },
    claimWheel: on => { calls.camera.push(`claimWheel:${on}`); },
    claimItems: () => undefined,
    held: () => undefined,
    mapping: () => undefined,
    clicksPerSecond: () => undefined,
  },
  'horizon:mods/ui': {
    setSettings: widgets => { calls.setSettings.push(widgets); },
    openScreen: () => undefined,
    closeScreen: () => undefined,
  },
};
const genericImport = new Proxy({}, { get: () => () => undefined });
const imports = new Proxy({}, {
  get: (_, name) => {
    if (typeof name !== 'string' || !name.includes('horizon:mods/')) return undefined;
    const key = name.replace(/@0\.1\.0$/, '');
    if (key.startsWith('horizon:mods/render')) {
      // Any render call would be an attempt to fake camera control.
      return new Proxy({}, { get: (_t, fn) => (...args) => {
        calls.camera.push(`${fn}(${JSON.stringify(args)})`);
      } });
    }
    return specificImports[key] ?? genericImport;
  },
});

const results = [];
const check = (label, actual, expected) => {
  const a = JSON.stringify(actual), e = JSON.stringify(expected);
  results.push({ label, ok: a === e, actual, expected });
};
const key = (tag, name) => { pending = [{ tag, val: name }]; };
const frame = (seconds = 1 / 60) => events.frame(seconds);
const frames = (n) => { for (let i = 0; i < n; i++) frame(); };
const label = (id) => calls.setSettings.at(-1)?.find(w => w.val?.id === id)?.val?.label;

const { events, uiEvents } = await instantiate(getCoreModule, imports);

// --- init -----------------------------------------------------------------
check('init registration', events.init(), {
  commands: ['freelook'], tick: false, frame: true, chat: false,
});
check('F5 key registered', calls.registerKey[0], {
  name: 'free_look', label: 'Free Look (F5)', key: 'key.keyboard.f5',
});
check('shift keys registered', calls.registerKey.slice(1).map(k => k.key), [
  'key.keyboard.left.shift', 'key.keyboard.right.shift',
]);
check('settings page size', calls.setSettings[0]?.length, 4);
check('Auto free look label', label(2), 'Auto free look: OFF');
check('Shift for free look label', label(3), 'Shift for free look: OFF');
check('warns in the game log', calls.log[0]?.message.includes('no camera-angle setter'), true);

// --- both settings off: F5 is the game's ----------------------------------
const chatBefore = calls.chat.length, hudBefore = calls.hud.length;
key('key-down', 'free_look'); frame(); frames(3);
key('key-up', 'free_look'); frame();
check('both off: no chat', calls.chat.length, chatBefore);
check('both off: no HUD panel', calls.hud.length, hudBefore);
check('both off: no settings written', calls.set.length, 0);

// --- turn Auto free look on ------------------------------------------------
uiEvents.settingsOpened();
uiEvents.select(2, 0);
check('auto saved', calls.set.at(-1), ['auto_free_look', 'true']);
check('auto label ON', label(2), 'Auto free look: ON');
key('key-down', 'free_look'); frame();
check('activation explains the limitation', calls.chat.at(-1),
  "Free Look: camera control is unavailable in this Horizon build. F5 keeps the game's perspective.");
check('activation shows HUD status', calls.hud.at(-1)?.lines[0]?.text,
  "Free Look: camera control is unavailable in this Horizon build. F5 keeps the game's perspective.");
check('status command', events.command('freelook', 'status').includes('state active'), true);
check('status command names the gap', events.command('freelook', 'status').includes('camera control is unavailable'), true);

// --- F5 again: smooth return and HUD cleared -------------------------------
key('key-up', 'free_look'); frame();
key('key-down', 'free_look'); frame();
check('returning state', events.command('freelook', 'status').includes('state returning'), true);
const clearBefore = calls.hudClear;
key('key-up', 'free_look'); frames(90);
check('smooth return finishes', events.command('freelook', 'status').includes('state off'), true);
check('HUD cleared after return', calls.hudClear > clearBefore, true);

// --- Shift for free look: plain F5 stays vanilla ---------------------------
uiEvents.settingsOpened();
uiEvents.select(3, 0);
check('shift saved', calls.set.at(-1), ['shift_for_free_look', 'true']);
const hudBeforePlain = calls.hud.length;
key('key-down', 'free_look'); frame(); frames(3);
key('key-up', 'free_look'); frame();
check('plain F5 ignored with Shift ON', calls.hud.length, hudBeforePlain);
key('key-down', 'shift_left'); frame();
key('key-down', 'free_look'); frame();
check('Shift+F5 activates', events.command('freelook', 'status').includes('state active'), true);
pending = [
  { tag: 'key-up', val: 'free_look' },
  { tag: 'key-up', val: 'shift_left' },
];
frames(90);

// --- commands and reset ----------------------------------------------------
check('auto off command', events.command('freelook', 'auto off'), 'Auto free look set to OFF.');
check('unused verb is refused', (() => {
  try { events.command('freelook', 'nope'); return false; }
  catch (e) { return String(e).includes('Usage: /freelook'); }
})(), true);
check('unknown command is refused', (() => {
  try { events.command('other', ''); return false; }
  catch (e) { return String(e).includes('Unknown command.'); }
})(), true);
check('reset command', events.command('freelook', 'reset'),
  'Free Look settings reset (both OFF; F5 is vanilla\'s).');
check('reset saved both flags', calls.set.slice(-2), [
  ['auto_free_look', 'false'], ['shift_for_free_look', 'false'],
]);

// --- simulated restart: settings come back from storage --------------------
store.set('auto_free_look', enc.encode('true'));
store.set('shift_for_free_look', enc.encode('false'));
const second = await instantiate(getCoreModule, imports);
second.events.init();
check('settings persist across restart', [
  calls.setSettings.at(-1)?.find(w => w.val?.id === 2)?.val?.label,
  calls.setSettings.at(-1)?.find(w => w.val?.id === 3)?.val?.label,
], ['Auto free look: ON', 'Shift for free look: OFF']);

// --- no invented camera calls ---------------------------------------------
check('no camera function was called', calls.camera, []);

const failed = results.filter(r => !r.ok);
for (const r of results) {
  console.log(`${r.ok ? 'ok  ' : 'FAIL'} ${r.label}`);
  if (!r.ok) console.log(`     got ${r.actual}\n     expected ${r.expected}`);
}
console.log(`\n${results.length - failed.length}/${results.length} checks passed`);
if (failed.length) process.exit(1);

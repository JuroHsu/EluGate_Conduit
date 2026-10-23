import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import ts from 'typescript';

const source = await readFile(new URL('../src/language-sync.ts', import.meta.url), 'utf8');
const { outputText } = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } });
const { syncLanguage } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`);
const tick = () => new Promise(resolve => setImmediate(resolve));

test('loads Traditional Chinese on first launch after subscribing', async () => {
  const order = [];
  const values = [];
  const dispose = syncLanguage(async () => { order.push('read'); return 'zh-TW'; }, async () => {
    order.push('subscribe'); return () => {};
  }, lang => values.push(lang));
  await tick();
  assert.deepEqual(order, ['subscribe', 'read']);
  assert.deepEqual(values, ['zh-TW']);
  dispose();
});

test('a late English snapshot cannot overwrite a language change', async () => {
  let resolve;
  let change;
  const values = [];
  const initial = new Promise(done => { resolve = done; });
  const dispose = syncLanguage(() => initial, async fn => { change = fn; return () => {}; }, lang => values.push(lang));
  await tick();
  change('zh-TW');
  resolve('en');
  await tick();
  assert.deepEqual(values, ['zh-TW']);
  dispose();
});

test('StrictMode cleanup unsubscribes a pending registration and ignores old replies', async () => {
  let resolve;
  let unsubscribed = 0;
  const values = [];
  const registration = new Promise(done => { resolve = done; });
  const dispose = syncLanguage(async () => 'en', () => registration, lang => values.push(lang));
  dispose();
  resolve(() => { unsubscribed++; });
  await tick();
  assert.equal(unsubscribed, 1);
  assert.deepEqual(values, []);
});

test('all mounted windows receive repeated selection of the same language', async () => {
  const handlers = new Set();
  const values = [[], []];
  const dispose = values.map(target => syncLanguage(async () => 'en', async handler => {
    handlers.add(handler); return () => handlers.delete(handler);
  }, lang => target.push(lang)));
  await tick();
  for (let i = 0; i < 2; i++) handlers.forEach(handler => handler('zh-TW'));
  assert.deepEqual(values, [['en', 'zh-TW', 'zh-TW'], ['en', 'zh-TW', 'zh-TW']]);
  dispose.forEach(fn => fn());
  assert.equal(handlers.size, 0);
});

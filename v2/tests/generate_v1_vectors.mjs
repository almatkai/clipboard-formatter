// Optional regeneration: built-in Node modules only; never calls the OS clipboard.
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { createHash } from 'node:crypto';
import vm from 'node:vm';

const fixture = new URL('./fixtures/v1-index.ts', import.meta.url);
const live = new URL('../../src/index.ts', import.meta.url);
const source = readFileSync(fixture, 'utf8');
if (existsSync(live) && readFileSync(live, 'utf8') !== source) {
  throw new Error('V1 source no longer matches the pinned oracle input');
}
// Execute the complete untouched command after removing only import/export and
// its one TypeScript annotation. The regex and trim expressions are unmodified.
const executable = source
  .replace(/^import .*;\r?\n/, '')
  .replace('export default async function Command()', 'async function Command()')
  .replace('catch (error: unknown)', 'catch (error)') + '\nCommand';

const inputs = [
  ['empty', ''], ['unchanged', 'already plain'], ['space-only', ' \t\r\n '],
  ['tags-only', '<b></b>'], ['unclosed-tag-only', '<'],
  ['basic', '  <b>Hello</b> <i>world</i>  '], ['entities', '&lt;b&gt; &amp; &#160; &#x41;'],
  ['unicode', '<p>Қазақ 日本 😀 e\u0301</p>'],
  ['unmatched-less-than', 'before <unclosed\nrest'], ['unmatched-greater-than', '>a>>'],
  ['nested-less-than', 'a <<b>c>d'], ['quoted-greater-than', 'a<tag title="x>y">b</tag>c'],
  ['multiline-tag', 'a<tag\nattr=\"v\"\r\n>b</tag>c'],
  ['comment-regex', 'a<!-- x > y -->b'], ['math-is-regex', '1 < 2 and 3 > 1'],
  ['script-remains-text', '<script>alert("x")</script>'],
  ['cr-runs', 'a\r\r\rb'], ['lf-runs', 'a\n\n\nb'],
  ['mixed-runs', '\r\na\r\n\n\r\rb\n\r'],
  ['spaces-separate-newline-runs', 'a\r\n \r\nb'],
  ['tag-removal-joins-runs', 'a\r<b>\n</b>\rb'],
  ['tag-removal-joins-text', 'a<b>b</b>c'], ['empty-tags', 'a<>b<>c'],
  ['adjacent-unclosed', 'a<b>c<d'], ['nul', '\0<b>x</b>\0'],
  ['internal-unicode-line-separators', 'a\u2028\u2028\u2029b'],
  ['internal-non-crlf-whitespace', 'a\v\v\f\f\tb'],
  ['bom-only', '\ufeff'], ['nel-only', '\u0085'], ['bom-and-nel', '\ufeff\u0085x\u0085\ufeff'],
];
const whitespace = [0x9,0xa,0xb,0xc,0xd,0x20,0xa0,0x1680,
  ...Array.from({length:11},(_,i)=>0x2000+i),0x2028,0x2029,0x202f,0x205f,0x3000,0xfeff];
for (const code of whitespace) {
  const c = String.fromCodePoint(code);
  inputs.push([`js-trim-${code.toString(16)}`, `${c}<b>x</b>${c}`]);
}
for (const code of [0x85,0x180e,0x200b,0x2060,0x1c]) {
  const c = String.fromCodePoint(code);
  inputs.push([`not-js-trim-${code.toString(16)}`, `${c}<b>x</b>${c}`]);
}

async function execute(input, failAt, thrown) {
  const calls = [], effects = [];
  const command = vm.runInNewContext(executable, {
    Error,
    Clipboard: {
      async read() {
        calls.push(['clipboard:read', []]);
        if (failAt === 'read') throw thrown;
        return { text: input };
      },
      async copy(text) {
        calls.push(['clipboard:write', [text]]);
        if (failAt === 'write') throw thrown;
      },
    },
    Toast: { Style: { Failure: 'failure' } },
    showToast(options) { effects.push({ kind: 'toast', ...options }); },
    async showHUD(title) { effects.push({ kind: 'hud', title }); },
  });
  await command();
  return { calls, effects };
}
const vectors = [];
for (const [name, input] of inputs) vectors.push({ name, input, ...await execute(input) });
const failures = [];
for (const failAt of ['read', 'write']) {
  for (const kind of ['error', 'non-error']) {
    const thrown = kind === 'error' ? new Error(`${failAt} denied`) : 'not an Error';
    failures.push({ name: `${failAt}-${kind}`, input: '<b>secret</b>', failAt, kind,
      ...await execute('<b>secret</b>', failAt, thrown) });
  }
}
writeFileSync(new URL('./fixtures/v1-vectors.json', import.meta.url), JSON.stringify({
  sourceSha256: createHash('sha256').update(source).digest('hex'), vectors, failures,
}, null, 2) + '\n');
console.log(`Generated ${vectors.length} vectors and ${failures.length} failure cases from V1`);

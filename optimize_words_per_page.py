#!/usr/bin/env python3
"""Find the largest words_per_page values that keep each gloss table on its text's page.

For each displayed text in the sequence, page by page: typeset the page with a range
of word counts (one xelatex run per batch), keep the largest count whose gloss table
still lands on page 1, then start the next page right after those words. When the rest
of the text fits on one page, that is the last page.

The Rust side needs the private make_page(), so the crate's source is copied to a temp
dir and two env-driven test helpers are appended there; src/lib.rs is not modified.
Build artifacts are cached in target/glossopt.

usage: ./optimize_words_per_page.py [--seq ../gkvocab_data/testsequence.xml]
                                    [--texts phaedrus.xml ajax.xml] [--jobs 5] [--no-verify]
"""
import argparse
import collections
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import threading
from concurrent.futures import ThreadPoolExecutor

PROJECT = os.path.dirname(os.path.abspath(__file__))
MAX_WORDS = 400  # largest page tried; a much larger gloss table gives "Dimension too large"
COARSE_STEP = 10

RUST_HELPERS = r'''
#[cfg(test)]
mod glossopt {
    use super::*;
    use exportlatex::ExportLatex;

    fn options() -> GlossPageOptions {
        GlossPageOptions { filter_unique: true, filter_invisible: true, sort_key: true }
    }

    // env: SEQ, TEXT (file name), OFFSET, LO, HI, STEP, OUT
    // writes "%%LEN n" then one LaTeX page per candidate word count, each after "%%CAND w"
    #[test]
    fn glossopt_candidates() {
        let Ok(seq_path) = std::env::var("SEQ") else { return };
        let Ok(text) = std::env::var("TEXT") else { return };
        let var = |k: &str| -> usize { std::env::var(k).unwrap().parse().unwrap() };
        let (offset, lo, hi, step) = (var("OFFSET"), var("LO"), var("HI"), var("STEP"));

        let seq = Sequence::from_xml(&seq_path).unwrap();
        let go = seq.process().unwrap();
        let mut appcrit_hash = HashMap::default();
        for t in &seq.texts {
            if let Some(appcrits) = &t.appcrits {
                for ap in appcrits {
                    appcrit_hash.insert(ap.word_uuid, ap.entry.clone());
                }
            }
        }
        let t_idx = seq.sequence_description.texts.iter().position(|t| t.text == text).unwrap();
        let words = &go[t_idx];
        let title = if offset == 0 { "" } else { seq.texts[t_idx].text_name.as_str() };
        let mut res = format!("%%LEN {}\n", words.len());
        let mut w = lo;
        while w <= hi && offset + w <= words.len() {
            let mut idx = vec![];
            res.push_str(&format!("%%CAND {}\n", w));
            res.push_str(&make_page(&words[offset..offset + w], &appcrit_hash, &ExportLatex {}, title, &mut idx, 1, &options()));
            w += step;
        }
        fs::write(std::env::var("OUT").unwrap(), res).unwrap();
    }

    // env: SEQ, OUT, optional WPP (lines of "text file name<TAB>words_per_page")
    #[test]
    fn glossopt_document() {
        let Ok(seq_path) = std::env::var("SEQ") else { return };
        let mut seq = Sequence::from_xml(&seq_path).unwrap();
        if let Ok(wpp) = std::env::var("WPP") {
            for line in wpp.lines() {
                let (name, values) = line.split_once('\t').unwrap();
                for t in seq.sequence_description.texts.iter_mut() {
                    if t.text == name {
                        t.words_per_page = values.to_string();
                    }
                }
            }
        }
        let go = seq.process().unwrap();
        let doc = seq.make_document(&go, &ExportLatex {}, &options());
        fs::write(std::env::var("OUT").unwrap(), doc).unwrap();
    }
}
'''

print_lock = threading.Lock()


def log(msg):
    with print_lock:
        print(msg, flush=True)


def build_helper(work):
    """copy the crate source, append the helpers, build; returns the test binary path"""
    crate = os.path.join(work, 'crate')
    os.makedirs(crate)
    # top-level files too: the crate include_bytes!()s the font and collation blob
    for f in os.listdir(PROJECT):
        path = os.path.join(PROJECT, f)
        if os.path.isfile(path) and not f.endswith(('.pdf', '.log', '.aux')):
            shutil.copy(path, crate)
    shutil.copytree(os.path.join(PROJECT, 'src'), os.path.join(crate, 'src'))
    with open(os.path.join(crate, 'src', 'lib.rs'), 'a', encoding='utf-8') as f:
        f.write(RUST_HELPERS)
    env = dict(os.environ, CARGO_TARGET_DIR=os.path.join(PROJECT, 'target', 'glossopt'))
    log('building helper (first build compiles dependencies, later ones are quick)...')
    out = subprocess.run(['cargo', 'test', '--release', '--no-run', '--message-format=json-render-diagnostics'],
                         cwd=crate, env=env, stdout=subprocess.PIPE, check=True).stdout
    for line in out.decode().splitlines():
        msg = json.loads(line)
        if msg.get('reason') == 'compiler-artifact' and msg.get('executable') \
                and msg['target']['name'] == 'gkvocab_test':
            return msg['executable']
    sys.exit('could not find the test binary')


def run_helper(binary, test, **env):
    env = dict(os.environ, **{k: str(v) for k, v in env.items()})
    subprocess.run([binary, '--exact', test, '--quiet'], env=env, check=True,
                   stdout=subprocess.DEVNULL)


def xelatex(tex_path, outdir):
    """compile from the project dir so the relative font paths in the preamble resolve"""
    subprocess.run(['xelatex', '-interaction=nonstopmode', f'-output-directory={outdir}', tex_path],
                   cwd=PROJECT, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    log_text = open(os.path.splitext(tex_path)[0] + '.log', encoding='utf-8', errors='replace').read()
    # \letcs already defined also happens in the real build
    errors = [e for e in re.findall(r'(?m)^! .*', log_text) if '\\letcs already defined' not in e]
    return log_text, errors


def mark_tables(tex):
    """log the page each gloss table is printed on (inside the float, so layout is unchanged)"""
    n = 0

    def rep(m):
        nonlocal n
        n += 1
        return m.group(0) + '\\write-1{TABLEPAGE %d \\thepage}' % n
    return re.sub(r'(?m)^(?!%)(?:[^\n%]*?)?\\begin\{table\}\[b!\]', rep, tex)


class TextOptimizer:
    def __init__(self, binary, seq, text, preamble, work):
        self.binary, self.seq, self.text, self.preamble = binary, seq, text, preamble
        self.work = os.path.join(work, re.sub(r'\W', '_', text))
        os.makedirs(self.work)

    def candidates(self, offset, lo, hi, step=1):
        out = os.path.join(self.work, 'cand.tex')
        run_helper(self.binary, 'glossopt::glossopt_candidates', SEQ=self.seq, TEXT=self.text,
                   OFFSET=offset, LO=lo, HI=hi, STEP=step, OUT=out)
        body = open(out, encoding='utf-8').read()
        return int(re.match(r'%%LEN (\d+)', body).group(1)), body

    def fits(self, body):
        """typeset each candidate on its own page 1; it fits if its table is on page 1"""
        parts = re.split(r'(?m)^%%CAND (\d+)\n', body)[1:]
        doc = self.preamble
        ws = []
        for w, page in zip(parts[::2], parts[1::2]):
            ws.append(int(w))
            page = page.replace('\\begin{table}[b!]',
                                '\\begin{table}[b!]\\write-1{CAND %s \\thepage}' % w, 1)
            doc += '\\clearpage\\setcounter{page}{1}\n' + page
        doc += '\\end{document}\n'
        tex = os.path.join(self.work, 'c.tex')
        open(tex, 'w', encoding='utf-8').write(doc)
        log_text, errors = xelatex(tex, self.work)
        if errors:
            raise RuntimeError(f'{self.text}: LaTeX error {errors[0]!r}, see {self.work}/c.log')
        pages = dict(re.findall(r'CAND (\d+) (\S+)', log_text))
        return {w: pages.get(str(w)) == '1' for w in ws}

    def run(self):
        total, _ = self.candidates(0, 1, 0)
        offset, result = 0, []
        while offset < total:
            rest = total - offset
            if rest <= MAX_WORDS and self.fits(self.candidates(offset, rest, rest)[1])[rest]:
                result.append(rest)
                log(f'{self.text} page {len(result)}: {rest} words (last page)')
                break
            f = self.fits(self.candidates(offset, COARSE_STEP, MAX_WORDS, COARSE_STEP)[1])
            ok = [w for w in f if f[w]]
            if not ok:
                raise RuntimeError(f'{self.text}: not even {COARSE_STEP} words fit at word {offset}')
            best = max(ok)
            f = self.fits(self.candidates(offset, best + 1, best + COARSE_STEP - 1)[1])
            best = max([best] + [w for w in f if f[w]])
            result.append(best)
            offset += best
            log(f'{self.text} page {len(result)}: {best} words')
        return result


def verify(binary, seq, results, work):
    """compile the whole document with the new values and check every table's page"""
    log('verifying: compiling the full document with the new values...')
    tex = os.path.join(work, 'verify.tex')
    wpp = '\n'.join(f'{t}\t{", ".join(map(str, v))}' for t, v in results.items())
    run_helper(binary, 'glossopt::glossopt_document', SEQ=seq, OUT=tex, WPP=wpp)
    marked = mark_tables(open(tex, encoding='utf-8').read())
    open(tex, 'w', encoding='utf-8').write(marked)
    log_text, errors = xelatex(tex, work)
    pages = [int(p) for p in re.findall(r'TABLEPAGE \d+ (\d+)', log_text)]
    doubled = [p for p, c in collections.Counter(pages).items() if c > 1]
    gaps = [(a, b) for a, b in zip(pages, pages[1:]) if b != a + 1]
    written = re.search(r'Output written on .*\((\d+) pages?\)', log_text)
    log(f'{written.group(1) if written else "?"} pages, {len(pages)} gloss tables')
    ok = not (errors or doubled or gaps)
    if errors:
        log(f'LaTeX errors: {errors}')
    if doubled or gaps:
        log(f'tables off their page: pages with two tables {doubled}, pages without one after {gaps}')
    if ok:
        log('verified: every gloss table is on its own text page')
    return ok


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--seq', default=os.path.join(PROJECT, '..', 'gkvocab_data', 'testsequence.xml'))
    ap.add_argument('--texts', nargs='*', help='text file names (default: every displayed text)')
    ap.add_argument('--jobs', type=int, default=os.cpu_count() or 4)
    ap.add_argument('--no-verify', action='store_true', help='skip the full-document check')
    ap.add_argument('--keep', action='store_true', help='keep the temp work dir')
    args = ap.parse_args()
    seq = os.path.abspath(args.seq)

    xml = open(seq, encoding='utf-8').read()
    displayed = re.findall(r'<text display="true" file_name="([^"]+)"', xml)
    texts = args.texts or displayed
    for t in texts:
        if t not in displayed:
            sys.exit(f'{t} is not a displayed text in {seq}')

    work = tempfile.mkdtemp(prefix='glossopt-')
    try:
        binary = build_helper(work)
        doc = os.path.join(work, 'doc.tex')
        run_helper(binary, 'glossopt::glossopt_document', SEQ=seq, OUT=doc)
        src = open(doc, encoding='utf-8').read()
        preamble = src[:src.index('\\begin{document}') + len('\\begin{document}\n')]

        with ThreadPoolExecutor(max_workers=args.jobs) as pool:
            futures = {t: pool.submit(TextOptimizer(binary, seq, t, preamble, work).run) for t in texts}
            results = {t: f.result() for t, f in futures.items()}

        ok = True if args.no_verify else verify(binary, seq, results, work)
        print()
        for t, v in results.items():
            print(f'{t} ({len(v)} pages):\n  {", ".join(map(str, v))}')
        if not ok:
            sys.exit(1)
    finally:
        if args.keep:
            print(f'work dir: {work}')
        else:
            shutil.rmtree(work, ignore_errors=True)


if __name__ == '__main__':
    main()

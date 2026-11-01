#!/usr/bin/env python3
"""Find the largest words_per_page values that keep each gloss table on its text's page.

For each displayed text in the sequence, page by page: typeset the page with a range
of word counts (one xelatex run per batch), keep the largest count whose gloss table
still lands on page 1, then start the next page right after those words. When the rest
of the text fits on one page, that is the last page.

A page never starts with punctuation, never ends on a section number, paragraph or
speaker marker, never ends inside <...>, [...] or (...), and in verse only ends at the
end of a verse line. With --full-lines,
a prose page also has to end on a full line: the next word would have started a new
line (LaTeX's \\prevgraf goes up), or the text starts a new line there anyway.

The Rust side needs the private make_page(), so the crate's source is copied to a temp
dir and env-driven test helpers are appended there; src/lib.rs is not modified.
Build artifacts are cached in target/glossopt.

usage: ./optimize_words_per_page.py [--seq ../gkvocab_data/testsequence.xml]
                                    [--texts phaedrus.xml ajax.xml] [--jobs 5]
                                    [--full-lines] [--no-verify]
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

    // env: SEQ, TEXT (file name), OFFSET, WS (comma separated word counts), OUT
    // writes "%%LEN n" then one LaTeX page per candidate word count, each after "%%CAND w"
    #[test]
    fn glossopt_candidates() {
        let Ok(seq_path) = std::env::var("SEQ") else { return };
        let Ok(text) = std::env::var("TEXT") else { return };
        let offset: usize = std::env::var("OFFSET").unwrap().parse().unwrap();
        let ws: Vec<usize> = std::env::var("WS").unwrap().split(',')
            .filter_map(|s| s.trim().parse().ok()).collect();

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
        for w in ws.into_iter().filter(|w| offset + w <= words.len()) {
            let mut idx = vec![];
            res.push_str(&format!("%%CAND {}\n", w));
            res.push_str(&make_page(&words[offset..offset + w], &appcrit_hash, &ExportLatex {}, title, &mut idx, 1, &options(),
                                    paragraph_continues(&words[offset + w..])));
        }
        fs::write(std::env::var("OUT").unwrap(), res).unwrap();
    }

    // env: SEQ, TYPES_TEXT (file name), OUT; writes each word's "WordType<TAB>word", one per line
    #[test]
    fn glossopt_types() {
        let Ok(seq_path) = std::env::var("SEQ") else { return };
        let Ok(text) = std::env::var("TYPES_TEXT") else { return };
        let seq = Sequence::from_xml(&seq_path).unwrap();
        let go = seq.process().unwrap();
        let t_idx = seq.sequence_description.texts.iter().position(|t| t.text == text).unwrap();
        let types: Vec<String> = go[t_idx].iter()
            .map(|g| format!("{:?}\t{}", g.word.word_type, g.word.word.replace(['\t', '\n'], " ")))
            .collect();
        fs::write(std::env::var("OUT").unwrap(), types.join("\n")).unwrap();
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


# word types after which the typeset text starts a new line
LINE_STARTS = {'ParaWithIndent', 'InlineSpeaker', 'WorkTitle', 'SectionTitle', 'Speaker'}
# in a verse section, a page may only end right before one of these
VERSE_BREAKS = {'VerseLine', 'Speaker', 'WorkTitle', 'SectionTitle'}
# a page must end on one of these, not on a section number, paragraph or speaker marker
PAGE_END_TYPES = {'Word', 'Punctuation'}
BREAKS_PER_BATCH = 30  # allowed break points typeset per xelatex run when choosing a page end


class TextOptimizer:
    def __init__(self, binary, seq, text, preamble, work, full_lines):
        self.binary, self.seq, self.text, self.preamble = binary, seq, text, preamble
        self.full_lines = full_lines
        self.work = os.path.join(work, re.sub(r'\W', '_', text))
        os.makedirs(self.work)
        out = os.path.join(self.work, 'types')
        run_helper(binary, 'glossopt::glossopt_types', SEQ=seq, TYPES_TEXT=text, OUT=out)
        rows = [line.split('\t', 1) for line in open(out, encoding='utf-8').read().split('\n')]
        self.types = [r[0] for r in rows]
        self.total = len(self.types)
        # bracket depth after each word, so a page never ends inside <...>, [...] or (...)
        # (only visible words count: verse line numbers like "172 [Str." aren't brackets)
        self.depth, d = [], 0
        for t, w in rows:
            if t in PAGE_END_TYPES:
                for ch in w:
                    if ch in '<[(':
                        d += 1
                    elif ch in '>])':
                        d = max(0, d - 1)
            self.depth.append(d)
        # mirrors make_text(): a verse line marker starts verse, a paragraph goes back to prose
        self.in_verse, v = [], False
        for t in self.types:
            if t == 'VerseLine':
                v = True
            elif t in ('ParaWithIndent', 'ParaNoIndent'):
                v = False
            self.in_verse.append(v)

    def visible_type(self, i, step):
        """type of the first word from i (step 1) or before i (step -1) that make_text() prints"""
        if step < 0:
            i -= 1
        while 0 <= i < self.total:
            if self.types[i] != 'InvalidType':
                return self.types[i]
            i += step
        return None

    def can_break(self, end):
        """may a page end just before word index end?"""
        if end >= self.total:
            return True
        last, nxt = self.visible_type(end, -1), self.visible_type(end, 1)
        if nxt is None:
            return True
        if last not in PAGE_END_TYPES or nxt == 'Punctuation' or self.depth[end - 1] > 0:
            return False
        if self.in_verse[end - 1]:
            return nxt in VERSE_BREAKS
        return True

    def next_line_end(self, end):
        """for a prose page ending before word index end: the index after the next word (with
        the punctuation attached to it), to see if that word would have started a new line.
        None if the text starts a new line there anyway."""
        if end >= self.total or self.in_verse[end - 1]:
            return None
        i = end
        while i < self.total and self.types[i] != 'Word':
            if self.types[i] in LINE_STARTS:
                return None
            i += 1
        if i == self.total:
            return None
        i += 1
        while i < self.total and self.types[i] in ('Punctuation', 'InvalidType'):
            i += 1
        return i

    def measure(self, offset, ws):
        """typeset a page of each word count in ws, each on its own page 1. returns
        ({w: table is on page 1}, {w: lines in the page's last prose paragraph})"""
        ws = sorted(set(w for w in ws if 0 < w <= self.total - offset))
        out = os.path.join(self.work, 'cand.tex')
        run_helper(self.binary, 'glossopt::glossopt_candidates', SEQ=self.seq, TEXT=self.text,
                   OFFSET=offset, WS=','.join(map(str, ws)), OUT=out)
        body = open(out, encoding='utf-8').read()
        parts = re.split(r'(?m)^%%CAND (\d+)\n', body)[1:]
        doc = self.preamble
        for w, page in zip(parts[::2], parts[1::2]):
            page = page.replace('\\begin{table}[b!]',
                                '\\begin{table}[b!]\\write-1{CAND %s \\thepage}' % w, 1)
            # prose text ends with \hspace*{\fill} (or a justified last line when the paragraph
            # continues on the next page): end the paragraph there and log its line count
            # (\edef so \prevgraf is read right away: a plain \immediate\write here reads 0)
            count = ('\\edef\\glossoptlines{\\noexpand\\immediate\\noexpand\\write-1'
                     '{LINES %s \\the\\prevgraf}}\\glossoptlines' % w)
            page = page.replace('\\hspace*{\\fill}\n\\end{spacing}',
                                '\\hspace*{\\fill}\\par' + count + '\n\\end{spacing}', 1)
            page = page.replace('{\\parfillskip=0pt\\par}\n\\end{spacing}',
                                '{\\parfillskip=0pt\\par}' + count + '\n\\end{spacing}', 1)
            doc += '\\clearpage\\setcounter{page}{1}\n' + page
        doc += '\\end{document}\n'
        tex = os.path.join(self.work, 'c.tex')
        open(tex, 'w', encoding='utf-8').write(doc)
        log_text, errors = xelatex(tex, self.work)
        if errors:
            raise RuntimeError(f'{self.text}: LaTeX error {errors[0]!r}, see {self.work}/c.log')
        pages = dict(re.findall(r'CAND (\d+) (\S+)', log_text))
        lines = {int(w): int(n) for w, n in re.findall(r'LINES (\d+) (\d+)', log_text)}
        return {w: pages.get(str(w)) == '1' for w in ws}, lines

    def choose(self, offset, limit):
        """the largest allowed page end at or below limit words that fits (and, with
        full_lines, ends on a full line). returns (words, ended on a full line)"""
        breaks = [w for w in range(limit, 0, -1) if self.can_break(offset + w)]
        first_fit = None
        # a full-line ending is normally within the last line or two; give up after 2 batches
        # rather than fall back to a much shorter page
        for i in range(0, min(len(breaks), 2 * BREAKS_PER_BATCH), BREAKS_PER_BATCH):
            batch = breaks[i:i + BREAKS_PER_BATCH]
            nexts = {}
            if self.full_lines:
                for w in batch:
                    n = self.next_line_end(offset + w)
                    if n is not None:
                        nexts[w] = n - offset
            fit, lines = self.measure(offset, batch + list(nexts.values()))
            for w in batch:
                if not fit.get(w):
                    continue
                if first_fit is None:
                    first_fit = w
                if w not in nexts or lines.get(nexts[w], 0) > lines.get(w, 10 ** 9):
                    return w, True
            if first_fit is not None and not self.full_lines:
                break
        if first_fit is None:
            raise RuntimeError(f'{self.text}: no page end fits at word {offset}')
        return first_fit, False

    def run(self):
        offset, result, not_full = 0, [], []
        while offset < self.total:
            rest = self.total - offset
            if rest <= MAX_WORDS and self.measure(offset, [rest])[0][rest]:
                result.append(rest)
                log(f'{self.text} page {len(result)}: {rest} words (last page)')
                break
            # the most words that fit, wherever that breaks the text; then the best page end below it
            fit, _ = self.measure(offset, range(COARSE_STEP, MAX_WORDS + 1, COARSE_STEP))
            ok = [w for w in fit if fit[w]]
            if not ok:
                raise RuntimeError(f'{self.text}: not even {COARSE_STEP} words fit at word {offset}')
            fit, _ = self.measure(offset, range(max(ok) + 1, max(ok) + COARSE_STEP))
            limit = max(ok + [w for w in fit if fit[w]])
            w, full = self.choose(offset, limit)
            result.append(w)
            offset += w
            note = ''
            if self.full_lines and not full:
                not_full.append(len(result))
                note = ' (no full-line ending fits, ends mid-line)'
            log(f'{self.text} page {len(result)}: {w} words{note}')
        if not_full:
            log(f'{self.text}: pages not ending on a full line: {not_full}')
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
    # TeX wraps long log lines, so the path may span lines
    written = re.search(r'Output written on [^(]*\((\d+) pages?\)', log_text)
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
    ap.add_argument('--full-lines', action='store_true',
                    help='end each prose page on a full line (or a paragraph/speaker break), '
                         'even if that means fewer words')
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
            futures = {t: pool.submit(TextOptimizer(binary, seq, t, preamble, work, args.full_lines).run) for t in texts}
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

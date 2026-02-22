#!/usr/bin/env python3
"""Capture C++ voikko output for differential testing against Rust.

Generates golden JSON files for spell, analyze, and hyphenate operations.

Usage:
    python3 capture_cpp.py [dict_path] [wordlist]

Requires: libvoikko Python bindings (uses ctypes, looks for libvoikko.dylib)
"""
import json
import os
import sys
import ctypes

# Find libvoikko
LIBVOIKKO_DIR = os.path.join(os.path.dirname(__file__), '..', '..', '..', 'src', '.libs')
DICT_PATH = os.path.join(os.path.dirname(__file__), '..', '..', '..', '..', 'voikko-fi', 'vvfst')
WORDLIST = os.path.join(os.path.dirname(__file__), 'wordlist.txt')
GOLDEN_DIR = os.path.join(os.path.dirname(__file__), 'golden')

def main():
    dict_path = sys.argv[1] if len(sys.argv) > 1 else DICT_PATH
    wordlist_path = sys.argv[2] if len(sys.argv) > 2 else WORDLIST

    # Set up library path
    dylib_path = os.path.join(LIBVOIKKO_DIR, 'libvoikko.dylib')
    if not os.path.exists(dylib_path):
        # Try .1.dylib
        dylib_path = os.path.join(LIBVOIKKO_DIR, 'libvoikko.1.dylib')

    if not os.path.exists(dylib_path):
        print(f"Error: libvoikko not found in {LIBVOIKKO_DIR}", file=sys.stderr)
        sys.exit(1)

    # Add to DYLD path for Python libvoikko
    os.environ['DYLD_LIBRARY_PATH'] = LIBVOIKKO_DIR
    # Use ctypes directly since we may not have the Python bindings installed
    sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', '..', '..', 'python'))
    from libvoikko import Voikko

    # Read wordlist
    with open(wordlist_path, 'r') as f:
        words = [line.strip() for line in f if line.strip()]

    print(f"Testing {len(words)} words with dict at {dict_path}")

    # Initialize Voikko
    v = Voikko('fi', path=dict_path)

    # Spell check
    spell_results = {}
    for word in words:
        spell_results[word] = v.spell(word)
    print(f"  Spell: {sum(spell_results.values())}/{len(words)} correct")

    # Analyze
    analyze_results = {}
    for word in words:
        analyses = v.analyze(word)
        analyze_results[word] = [dict(a) for a in analyses]
    analyzed_count = sum(1 for a in analyze_results.values() if a)
    print(f"  Analyze: {analyzed_count}/{len(words)} have analyses")

    # Hyphenate
    hyphenate_results = {}
    for word in words:
        hyphenate_results[word] = v.hyphenate(word)
    print(f"  Hyphenate: done")

    # Suggest (only for misspelled words)
    suggest_results = {}
    for word in words:
        if not spell_results[word]:
            suggestions = v.suggest(word)
            suggest_results[word] = suggestions[:5]  # Top 5
    print(f"  Suggest: {len(suggest_results)} misspelled words")

    # Save golden files
    os.makedirs(GOLDEN_DIR, exist_ok=True)

    with open(os.path.join(GOLDEN_DIR, 'spell.json'), 'w') as f:
        json.dump(spell_results, f, ensure_ascii=False, indent=2, sort_keys=True)

    with open(os.path.join(GOLDEN_DIR, 'analyze.json'), 'w') as f:
        json.dump(analyze_results, f, ensure_ascii=False, indent=2, sort_keys=True)

    with open(os.path.join(GOLDEN_DIR, 'hyphenate.json'), 'w') as f:
        json.dump(hyphenate_results, f, ensure_ascii=False, indent=2, sort_keys=True)

    with open(os.path.join(GOLDEN_DIR, 'suggest.json'), 'w') as f:
        json.dump(suggest_results, f, ensure_ascii=False, indent=2, sort_keys=True)

    v.terminate()
    print(f"\nGolden files written to {GOLDEN_DIR}/")

if __name__ == '__main__':
    main()

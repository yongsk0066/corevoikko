# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 프로젝트 개요

Corevoikko는 핀란드어 자연어 처리 오픈소스 라이브러리로, 맞춤법 검사, 하이픈 처리, 문법 검사, 형태소 분석을 제공한다.

핵심 구현은 **Rust**로 재작성 완료 (Phase 0~5). C++ 원본은 `libvoikko/legacy/`에 보존.

라이선스: MPL 1.1 / GPL 2+ / LGPL 2.1+ (tri-license)

## 디렉토리 구조

```
corevoikko/
├── libvoikko/
│   ├── rust/                  # Rust 핵심 구현 (6 crates)
│   │   └── crates/
│   │       ├── voikko-core/   # 공유 타입 (68 tests)
│   │       ├── voikko-fst/    # VFST FST 엔진 (71 tests)
│   │       ├── voikko-fi/     # 핀란드어 모듈 (494 tests, 10 ignored)
│   │       ├── voikko-wasm/   # wasm-bindgen WASM (4 tests, 189KB)
│   │       ├── voikko-ffi/    # C FFI cdylib (420KB, 30+ 함수)
│   │       └── voikko-cli/    # Rust CLI 도구 (8개 바이너리)
│   ├── js/                    # ESM TypeScript 래퍼 (37 vitest)
│   ├── python/libvoikko.py    # Python ctypes → voikko-ffi
│   ├── java/.../VoikkoRust.java  # Java JNA → voikko-ffi
│   ├── cs/VoikkoRust.cs       # C# P/Invoke → voikko-ffi
│   ├── cl/voikko-rust.lisp    # Common Lisp CFFI → voikko-ffi
│   ├── legacy/                # C++ 원본 (보존용, legacy/cpp-backup 브랜치)
│   │   ├── cpp-src/           # C++ 소스
│   │   ├── cpp-test/          # C++ 테스트
│   │   ├── cs-cpp/            # 구 C# 바인딩
│   │   ├── autotools/         # configure.ac, autogen.sh, m4/
│   │   └── emscripten/        # 구 WASM 빌드 (libvoikko_api.js)
│   ├── doc/                   # 문서
│   └── data/                  # 문법 도움말 XML
├── voikko-fi/                 # 핀란드어 사전 데이터 (VFST)
├── tools/                     # CLI 유틸리티 (Python 스크립트)
├── tests/                     # 통합 테스트 데이터
└── plan/phase2-rust/          # Rust 포팅 설계 문서
```

## 빌드 명령어

### Rust (핵심)

```bash
cd libvoikko/rust
cargo test --all-features      # 637 tests (core 68 + fst 71 + fi 494 + wasm 4, 10 ignored)
cargo clippy --all-features -- -D warnings
cargo bench -p voikko-fi --features handle  # 7개 벤치마크
```

### WASM 빌드

```bash
cd libvoikko/rust
cargo build --target wasm32-unknown-unknown --release -p voikko-wasm
wasm-bindgen target/wasm32-unknown-unknown/release/voikko_wasm.wasm \
  --out-dir ../js/wasm --target web --typescript
wasm-opt ../js/wasm/voikko_wasm_bg.wasm -Oz --enable-bulk-memory \
  -o ../js/wasm/voikko_wasm_bg.wasm
```

### FFI 빌드 (Python/C#/CL/Java용)

```bash
cd libvoikko/rust
cargo build --release -p voikko-ffi
# → target/release/libvoikko_ffi.{dylib,so,dll}
```

### CLI 도구

```bash
cd libvoikko/rust
VOIKKO_DICT_PATH=/path/to/dict cargo run -p voikko-cli --bin voikko-spell
# 가용 바이너리: voikko-spell, voikko-suggest, voikko-analyze,
# voikko-hyphenate, voikko-tokenize, voikko-gc-pretty,
# voikko-baseform, voikko-readability
```

### JS/WASM 패키지

```bash
cd libvoikko/js
pnpm install && pnpm build    # TS 래퍼 빌드
pnpm test                     # 37 vitest (Tier 1: 구조, Tier 2: WASM 통합)
```

### voikko-fi (핀란드어 사전)

```bash
cd voikko-fi
make vvfst                    # foma, python3, GNU make 필요
make vvfst-install DESTDIR=~/.voikko
```

## Rust 아키텍처

### Cargo Workspace (6 crates)

- **voikko-core** — 공유 타입 (enums, Analysis, Token, GrammarError, character, case)
- **voikko-fst** — VFST FST 엔진 (header, transitions, symbols, flags, unweighted/weighted traversal)
- **voikko-fi** — 핀란드어 모듈 (morphology, speller, hyphenator, tokenizer, suggestion, grammar)
- **voikko-wasm** — wasm-bindgen WASM 래퍼 (189KB, 15개 메서드 + 14개 옵션 setter)
- **voikko-ffi** — C FFI cdylib (420KB, C 헤더 `include/voikko.h`, 30+ extern "C" 함수)
- **voikko-cli** — Rust CLI 도구 (8개 바이너리: spell, suggest, analyze, hyphenate, tokenize, gc-pretty, baseform, readability)

### 언어 바인딩 (5개)

| 언어 | 파일 | 메커니즘 |
|------|------|----------|
| JS/TS | `js/src/index.ts` | voikko-wasm (wasm-bindgen) |
| Python | `python/libvoikko.py` + `voikko-ffi/python/` | voikko-ffi (ctypes) |
| Java | `java/.../VoikkoRust.java` | voikko-ffi (JNA) |
| C# | `cs/VoikkoRust.cs` | voikko-ffi (P/Invoke) |
| Common Lisp | `cl/voikko-rust.lisp` | voikko-ffi (CFFI) |

### 사전 파일

- `voikko-fi/vvfst/mor.vfst` (3.8MB) — 형태소 분석 트랜스듀서
- `voikko-fi/vvfst/autocorr.vfst` (11KB) — 자동교정 트랜스듀서
- `voikko-fi/vvfst/index.txt` — 사전 메타데이터

### Legacy (C++ 원본)

C++ 소스는 `libvoikko/legacy/`에 격리. 전체 히스토리는 `legacy/cpp-backup` 브랜치에 보존.
- `legacy/cpp-src/` — C++ 소스 (voikko.h, 모듈별 .cpp/.hpp)
- `legacy/cpp-test/` — Python unittest 기반 C++ 테스트
- `legacy/autotools/` — configure.ac, autogen.sh, Makefile.am, m4/
- `legacy/emscripten/` — Emscripten WASM 빌드 (libvoikko_api.js, build.sh)

## 검증 방법

```bash
# Rust 전체 테스트 + clippy
cd libvoikko/rust && cargo test --all-features && cargo clippy --all-features -- -D warnings

# JS/TS 테스트
cd libvoikko/js && pnpm test

# Python FFI 검증
VOIKKO_FFI_LIB=libvoikko/rust/target/release/libvoikko_ffi.dylib \
  python3 -c "from voikko import Voikko; v = Voikko('voikko-fi/vvfst'); print(v.spell('koira'))"

# 벤치마크
VOIKKO_DICT_PATH=voikko-fi/vvfst cargo bench -p voikko-fi --features handle
```

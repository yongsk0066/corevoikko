# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 프로젝트 개요

Corevoikko는 핀란드어 자연어 처리 오픈소스 라이브러리로, 맞춤법 검사, 하이픈 처리, 문법 검사, 형태소 분석을 제공한다.

핵심 구현은 **Rust**로 재작성 완료 (Phase 0~5). C++ 원본은 `libvoikko/legacy/`에 보존.

npm 패키지: `@yongsk0066/voikko` (v0.4.0)

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
│   ├── js/                    # npm 패키지 @yongsk0066/voikko (37 vitest)
│   │   ├── src/               # Voikko 클래스 + wasm-loader + types
│   │   └── dict/              # 번들 사전 (mor.vfst 3.8MB)
│   ├── python/libvoikko.py    # Python ctypes → voikko-ffi
│   ├── java/.../VoikkoRust.java  # Java JNA → voikko-ffi
│   ├── cs/VoikkoRust.cs       # C# P/Invoke → voikko-ffi
│   ├── cl/voikko-rust.lisp    # Common Lisp CFFI → voikko-ffi
│   ├── legacy/                # C++ 원본 (보존용, legacy/cpp-backup 브랜치)
│   ├── doc/                   # 문서
│   └── data/                  # 문법 도움말 XML
├── voikko-fi/                 # 핀란드어 사전 데이터 (VFST)
├── plan/                      # 포팅 설계 문서 (15개 .md)
├── tools/                     # CLI 유틸리티 (Python 스크립트)
├── tests/                     # 통합 테스트 데이터
└── .github/workflows/         # CI + Release + CodeQL
```

## 빌드 명령어

### Rust (핵심)

```bash
cd libvoikko/rust
cargo fmt --all --check           # 포맷 검사
cargo test --all-features         # 637 tests
cargo clippy --all-features -- -D warnings
cargo audit                       # 의존성 취약점 스캔
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
pnpm install && pnpm build    # TS 래퍼 빌드 (dist/index.mjs 14KB)
pnpm test                     # 37 vitest (Tier 1: 구조, Tier 2: WASM 통합)
```

### voikko-fi (핀란드어 사전)

```bash
cd voikko-fi
make vvfst                    # foma, python3, GNU make 필요
make vvfst-install DESTDIR=~/.voikko
```

## CI/CD 파이프라인

| 워크플로우 | 트리거 | 내용 |
|-----------|--------|------|
| CI | push/PR to master | Rust (fmt+test+clippy+audit) + JS/WASM (build+test) |
| Release | tag v* | test → npm publish + GitHub Release |
| CodeQL | push/PR to master | JavaScript 보안 분석 |

## npm 패키지 (@yongsk0066/voikko)

### 사용법

```typescript
// Node.js — zero-config (사전 번들)
const voikko = await Voikko.init();

// Browser — zero-config (CDN에서 자동 fetch)
const voikko = await Voikko.init();

// Browser self-host
const voikko = await Voikko.init('fi', { dictionaryUrl: '/dict/', wasmUrl: '/voikko.wasm' });
```

### 주요 기능
- typed error classes: `VoikkoError`, `WasmLoadError`, `DictionaryLoadError`
- WASM + 사전 캐싱 (에러 시 자동 초기화)
- terminate guard (use-after-terminate 방지)
- CDN fallback (unpkg, 버전 자동 동기화)
- 빌드타임 버전 주입 (`__PKG_VERSION__` define)

## Rust 아키텍처

### Cargo Workspace (6 crates)

- **voikko-core** — 공유 타입 (enums, Analysis, Token, GrammarError, character, case)
- **voikko-fst** — VFST FST 엔진 (header, transitions, symbols, flags, unweighted/weighted traversal)
- **voikko-fi** — 핀란드어 모듈 (morphology, speller, hyphenator, tokenizer, suggestion, grammar)
- **voikko-wasm** — wasm-bindgen WASM 래퍼 (189KB, 15개 메서드 + 14개 옵션 setter)
- **voikko-ffi** — C FFI cdylib (420KB, C 헤더 `include/voikko.h`, 30+ extern "C" 함수)
- **voikko-cli** — Rust CLI 도구 (8개 바이너리)

### 사전 파일

- `voikko-fi/vvfst/mor.vfst` (3.8MB) — 형태소 분석 트랜스듀서
- `voikko-fi/vvfst/autocorr.vfst` (11KB) — 자동교정 트랜스듀서
- `voikko-fi/vvfst/index.txt` — 사전 메타데이터
- npm 패키지에도 `js/dict/`로 번들 포함

## 검증 방법

```bash
# Rust 전체 검사
cd libvoikko/rust && cargo fmt --all --check && cargo test --all-features && cargo clippy --all-features -- -D warnings && cargo audit

# JS/TS 테스트
cd libvoikko/js && pnpm test

# 벤치마크
VOIKKO_DICT_PATH=voikko-fi/vvfst cargo bench -p voikko-fi --features handle
```

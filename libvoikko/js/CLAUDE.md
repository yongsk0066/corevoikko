# CLAUDE.md — libvoikko JS/WASM 패키지

## 개요

libvoikko C++ 라이브러리의 Emscripten WASM 빌드를 ESM TypeScript 패키지로 래핑한 프로젝트.
기존 `libvoikko_api.js` (cwrap 기반 C→JS 브릿지)는 그대로 유지하고, TypeScript 래퍼(`Voikko` 클래스)를 상위에 추가.

## 스택

- **pnpm** — 패키지 매니저
- **tsdown** — 라이브러리 번들러 (rolldown 기반, .d.ts 생성)
- **vitest** — 테스트 프레임워크
- **ESM only** — CJS 미지원

## 디렉토리 구조

```
libvoikko/js/
├── src/
│   ├── index.ts          # Voikko 클래스 (메인 진입점)
│   ├── types.ts          # 타입 정의 (Analysis, Token, GrammarError 등)
│   └── wasm-loader.ts    # Emscripten 모듈 로더 + 사전 VFS 마운트
├── test/
│   └── voikko.test.ts    # vitest 테스트 (37개, qunit.html에서 변환)
├── wasm/                 # emcc 빌드 출력 (gitignore)
│   └── .gitkeep
├── dist/                 # tsdown 빌드 출력 (gitignore)
├── build.sh              # Emscripten 빌드 스크립트 (EXPORT_ES6=1)
├── configure.sh          # emconfigure 래퍼
├── libvoikko_api.js      # cwrap 바인딩 (--post-js로 WASM에 주입, 수정 금지)
├── commonjs-footer.js    # 레거시 CJS export (더 이상 사용 안 함)
└── qunit.html            # 레거시 브라우저 테스트 (레퍼런스용)
```

## 빌드 명령어

```bash
cd libvoikko/js

# TS 래퍼 빌드 (WASM 없이도 가능)
pnpm install
pnpm build          # → dist/index.js + dist/index.d.ts

# WASM 빌드 (Emscripten SDK 필요, 프로젝트 루트에서)
cd libvoikko
./autogen.sh
js/configure.sh
js/build.sh plain   # → js/wasm/libvoikko.mjs + js/wasm/libvoikko.wasm
```

## 테스트

```bash
cd libvoikko/js

# Tier 1: 타입/구조 테스트 (항상 실행 가능)
pnpm test

# Tier 2: 통합 테스트 (WASM + 사전 필요)
VOIKKO_DICT_PATH=/path/to/dict pnpm test
```

Tier 2 테스트는 `VOIKKO_DICT_PATH` 환경변수가 없으면 자동 skip.

## 핵심 아키텍처 결정

### --post-js 유지
`libvoikko_api.js`는 Emscripten의 `--post-js`로 Module 클로저 안에 주입된다.
이 파일은 `cwrap`, `getValue`, `UTF8ToString` 등 Emscripten 런타임 함수를 클로저 스코프에서 접근한다.
**이 파일을 분리하거나 수정하면 런타임 함수 접근이 깨지므로 건드리지 않는다.**

### TypeScript 래퍼는 thin wrapper
`src/index.ts`의 `Voikko` 클래스는 `Module.init()`이 반환하는 raw 객체를 감싸기만 한다.
비즈니스 로직은 모두 C++ → libvoikko_api.js에 있다.

### 사전 VFS 마운트
브라우저에서는 fetch, Node.js에서는 fs로 사전 파일을 로드한 후
Emscripten FS API(`Module.FS.mkdir`, `Module.FS.writeFile`)로 가상 파일시스템에 마운트한다.
경로: `/5/mor-standard/` (V5DictionaryLoader가 `{root}/5/mor-{variant}/` 패턴으로 탐색).

## 수정 시 주의사항

- `libvoikko_api.js` — 수정 금지. Emscripten 클로저 스코프에 의존.
- `build.sh` — emcc 플래그 변경 시 `EXPORTED_FUNCTIONS`, `EXPORTED_RUNTIME_METHODS` 확인.
- `src/types.ts`의 `RawVoikkoInstance` — `libvoikko_api.js`가 반환하는 객체와 1:1 대응해야 함.
- `wasm/` 디렉토리 — emcc 출력물. gitignore 대상.

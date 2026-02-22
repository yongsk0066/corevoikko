# CLAUDE.md — libvoikko JS/WASM 패키지

## 개요

Rust voikko-wasm (wasm-bindgen)을 ESM TypeScript 패키지로 래핑.
Voikko 클래스는 WasmVoikko를 직접 래핑하는 thin wrapper.

## 스택

- **pnpm** — 패키지 매니저
- **tsdown** — 라이브러리 번들러 (rolldown 기반, .d.ts 생성)
- **vitest** — 테스트 프레임워크
- **ESM only** — CJS 미지원

## 디렉토리 구조

```
libvoikko/js/
├── src/
│   ├── index.ts          # Voikko 클래스 (thin wrapper, 228줄)
│   ├── types.ts          # 타입 정의 (Analysis, Token, GrammarError 등)
│   └── wasm-loader.ts    # loadWasm, loadDict (Rust WASM 로딩)
├── test/
│   ├── voikko.test.ts    # vitest 테스트 (37개)
│   └── setup-dict.ts     # globalSetup: 모노레포 사전 자동 감지
├── wasm/                 # wasm-bindgen 출력 (voikko_wasm_bg.wasm, 189KB)
├── dist/                 # tsdown 빌드 출력
├── test.html             # 브라우저 테스트 페이지
└── package.json
```

## 빌드 명령어

```bash
cd libvoikko/js
pnpm install
pnpm build          # → dist/index.mjs + dist/index.d.mts
pnpm test           # 37 vitest
```

WASM 빌드는 `libvoikko/rust/`에서 실행 (자세한 내용은 루트 CLAUDE.md 참조).

## 테스트 구조

- **Tier 1** (2개): 모듈 export/구조 테스트 (항상 실행)
- **Tier 2 - integration** (21개): spell, suggest, grammar, analyze 등 (beforeAll)
- **Tier 2 - option setters** (14개): 격리된 인스턴스 (beforeEach)

사전을 찾을 수 없으면 Tier 2는 자동 skip.

## 아키텍처

### 초기화 파이프라인

```
options ──┬── loadWasm()  ──────┐
          │                     ├── new WasmVoikko(morData, autocorrData) ── Voikko
          └── loadDict()  ──────┘
```

- `loadWasm`: Rust WASM 모듈 dynamic import + init (Node.js: 바이트 직접 전달, 브라우저: auto-fetch)
- `loadDict`: 사전 파일 I/O (fetch 또는 fs, V5 구조 자동 감지)
- WASM 모듈은 첫 호출 후 캐싱 (모듈 레벨)

### Voikko 클래스 = thin wrapper

`src/index.ts`의 Voikko 클래스는 WasmVoikko(Rust)를 직접 래핑.
비즈니스 로직은 전부 Rust(voikko-fi)에 있음. TS 레이어는 순수 위임 + 타입 매핑.

### 사전 경로 해석
- **브라우저** (`dictionaryUrl`): `{url}/5/mor-standard/{file}` fetch
- **Node.js** (`dictionaryPath`): flat 또는 V5 구조 자동 감지

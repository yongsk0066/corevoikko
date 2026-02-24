# CLAUDE.md — libvoikko JS/WASM 패키지

## 개요

Rust voikko-wasm (wasm-bindgen)을 ESM TypeScript 패키지로 래핑.
npm: `@yongsk0066/voikko` (v0.4.0)

## 스택

- **pnpm** — 패키지 매니저
- **tsdown** — 라이브러리 번들러 (rolldown 기반, .d.ts 생성, `__PKG_VERSION__` define 주입)
- **vitest** — 테스트 프레임워크
- **ESM only** — CJS 미지원

## 디렉토리 구조

```
libvoikko/js/
├── src/
│   ├── index.ts          # Voikko 클래스 (thin wrapper, ~270줄)
│   ├── types.ts          # 타입 정의 (Analysis, Token, GrammarError 등)
│   └── wasm-loader.ts    # loadWasm, loadDict + 에러 클래스 + 캐싱
├── test/
│   ├── voikko.test.ts    # vitest 테스트 (37개)
│   └── setup-dict.ts     # globalSetup: 모노레포 사전 자동 감지
├── dict/                 # 번들 사전 (mor.vfst 3.8MB, autocorr.vfst, index.txt)
├── wasm/                 # wasm-bindgen 출력 (voikko_wasm_bg.wasm, 189KB)
├── dist/                 # tsdown 빌드 출력 (index.mjs 14KB)
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

## 아키텍처

### 초기화 파이프라인

```
options ──┬── loadWasm() (캐시) ──┐
          │                       ├── new WasmVoikko(morData, autocorrData) ── Voikko
          └── loadDict() (캐시) ──┘
```

- `loadWasm`: WASM 모듈 캐싱 (에러 시 캐시 초기화)
- `loadDict`: 사전 캐싱 (키: URL/path/bundled/cdn)
- Node.js: 번들 사전 자동 발견 (zero-config)
- Browser: unpkg CDN fallback (zero-config)

### 에러 클래스

- `VoikkoError` — 베이스 에러
- `WasmLoadError extends VoikkoError` — WASM 로드 실패
- `DictionaryLoadError extends VoikkoError` — 사전 로드 실패 (`.fileName` 포함)

### Voikko 클래스

- `#handle: WasmVoikko | null` + `#terminated` flag
- `ensureActive()` — terminate 후 메서드 호출 시 명확한 에러
- 비즈니스 로직은 전부 Rust(voikko-fi)에 있음. TS 레이어는 순수 위임 + 타입 매핑.

### 사전 경로 해석
- **Node.js** (기본): 번들 `dict/` 자동 발견
- **Node.js** (`dictionaryPath`): flat 또는 V5 구조 자동 감지
- **Browser** (기본): unpkg CDN flat fetch
- **Browser** (`dictionaryUrl`): `{url}/5/mor-standard/{file}` fetch

### CDN 버전 동기화
- `__PKG_VERSION__` → tsdown/vitest `define`으로 `package.json` version 자동 주입
- CDN URL: `https://unpkg.com/@yongsk0066/voikko@{version}/...`

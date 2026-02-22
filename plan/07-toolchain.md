# JS 패키지 툴체인 결정

확정일: 2026-02-21

## 확정 스택

| 항목 | 선택 | 비고 |
|------|------|------|
| 패키지 매니저 | **pnpm** | |
| 모듈 포맷 | **ESM only** | CJS는 레거시, 2026 기준 ESM 단독 |
| TS 빌드 도구 | **tsdown** | rolldown 기반 라이브러리 번들러. .d.ts 내장 생성 |
| 테스트 | **vitest** | ESM 네이티브, 빠름 |
| 사전 로딩 | **fetch 기반 lazy loading** | 브라우저에서 가장 유연 |
| 번들러 | **불필요** | Emscripten emcc가 WASM+글루 생성, tsdown이 TS 래퍼 빌드 |

## 런타임 의존성

**0개**. devDependencies만 존재.

```json
{
  "devDependencies": {
    "tsdown": "latest",
    "typescript": "^5.x",
    "vitest": "^3.x"
  },
  "dependencies": {}
}
```

## 빌드 파이프라인

```
[1단계: Emscripten]
C++ 소스 → emcc → libvoikko.wasm + libvoikko-glue.mjs
  - -s EXPORT_ES6=1
  - -s MODULARIZE=1
  - EXPORTED_FUNCTIONS 유지
  - --post-js 제거 (API 코드를 별도 TS 래퍼로 분리)

[2단계: tsdown]
src/index.ts (TS 래퍼) → tsdown → dist/index.mjs + dist/index.d.mts
  - Emscripten 글루를 import
  - Voikko 클래스 export
  - .d.ts 자동 생성

[사전 데이터]
voikko-fi/vvfst/ → dist/dict/ (복사)
  - mor.vfst (~3.9 MB, fetch로 lazy load)
  - autocorr.vfst (~11 KB)
```

## 패키지 구조 (목표)

```
libvoikko/js/
├── package.json
├── pnpm-lock.yaml
├── tsconfig.json
├── tsdown.config.ts
├── vitest.config.ts
├── src/
│   ├── index.ts          # 메인 진입점 (Voikko 클래스)
│   ├── types.ts          # 타입 정의
│   └── wasm.ts           # Emscripten 글루 로더
├── wasm/                  # Emscripten 빌드 결과물
│   ├── libvoikko.wasm
│   └── libvoikko-glue.mjs
├── dict/                  # 사전 데이터 (빌드 시 복사)
│   ├── mor.vfst
│   └── autocorr.vfst
├── dist/                  # tsdown 빌드 출력
│   ├── index.mjs
│   └── index.d.mts
├── test/
│   └── voikko.test.ts
├── build.sh               # emcc 빌드 (기존 수정)
└── configure.sh            # emconfigure (기존 유지)
```

## 사용자 API (목표)

```typescript
import { Voikko } from 'voikko';

// 브라우저: fetch로 사전 로드
const voikko = await Voikko.init('fi', {
  dictionaryUrl: '/dict/'
});

// Node.js: 파일 시스템에서 로드
const voikko = await Voikko.init('fi', {
  dictionaryPath: './dict/'
});

// 맞춤법 검사
voikko.spell('koira');        // true
voikko.spell('koirra');       // false

// 제안
voikko.suggest('koirra');     // ['koira', ...]

// 형태소 분석
voikko.analyze('koirien');    // [{ STRUCTURE: '...', CLASS: '...', ... }]

// 하이픈 처리
voikko.hyphenate('kissa');    // 'kis-sa'

// 문법 검사
voikko.grammarErrors('Minä olen joten kuten kaunis.', 'fi');

// 토크나이저
voikko.tokens('kissa ja koira');

// 정리
voikko.terminate();
```

## 선택 근거

### pnpm > npm
- 디스크 효율 (심볼릭 링크)
- strict 모드 (phantom dependency 방지)
- 속도

### tsdown > tsup
- rolldown 기반 (차세대 번들러)
- .d.ts 생성 내장
- package.json exports 자동 생성
- tsup과 유사한 사용법 (마이그레이션 용이)

### tsdown > rolldown 직접 사용
- rolldown은 앱 번들링용, tsdown은 라이브러리 번들링용
- .d.ts 생성을 위해 별도 tsc 불필요
- 라이브러리 패키지에 필요한 설정이 기본 내장

### vitest > jest
- ESM 네이티브 지원 (jest는 ESM 지원이 실험적)
- 빠른 실행 (Vite 기반)
- TypeScript 설정 최소

### ESM only > dual ESM+CJS
- 2026년 기준 Node.js 생태계 ESM 전환 완료
- CJS 호환 레이어 유지보수 불필요
- 패키지 설정 단순화

### fetch 기반 > embed/preload
- 브라우저에서 가장 유연 (CDN, 캐싱, 진행률 표시 가능)
- Node.js에서도 fs.readFile 폴백으로 지원 가능
- 사전 데이터(~4 MB)를 WASM에 내장하지 않아 초기 로딩 최적화

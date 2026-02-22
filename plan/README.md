# Corevoikko JS Porting & Modernization Plan

JS 환경 포팅 및 프로젝트 모던화를 위한 조사/계획 문서.

## 문서 구조

| 문서 | 내용 |
|------|------|
| [01-codebase-analysis.md](./01-codebase-analysis.md) | 현재 코드베이스 분석 (C++ 표준, 아키텍처, 메모리 관리) |
| [02-emscripten.md](./02-emscripten.md) | Emscripten (C++ -> WASM) 접근법 분석 |
| [03-rust-wasm.md](./03-rust-wasm.md) | Rust + wasm-bindgen 접근법 분석 |
| [04-typescript.md](./04-typescript.md) | Pure TypeScript 재구현 분석 |
| [05-zig-wasm.md](./05-zig-wasm.md) | Zig + WASM 접근법 분석 |
| [06-comparison.md](./06-comparison.md) | 종합 비교 및 추천 전략 |
| [07-toolchain.md](./07-toolchain.md) | JS 패키지 툴체인 결정 (확정) |

## 확정 스택

```
pnpm + tsdown + vitest + ESM only + fetch 로딩
```

## 추천 전략 요약

- **Phase 1** (1-2주): 기존 Emscripten 빌드(`libvoikko/js/`) 모던화 -> npm 패키지 배포
- **Phase 2** (장기): Rust 포팅 검토 -> 메모리 안전성 + WASM 최적화 + 기존 바인딩 호환

## 진행 일지

- 2026-02-21: 초기 분석 완료, 4개 접근법 병렬 분석
- 2026-02-21: 툴체인 확정 (pnpm + tsdown + vitest)
- 2026-02-22: **Phase 1 구현 완료** — ESM 전환, TS 래퍼, vitest 테스트, build.sh 수정
  - `pnpm build` 성공 (dist/index.js 6.78KB + dist/index.d.ts 7.74KB)
  - `pnpm test` 성공 (2 passed, 35 skipped — WASM 미설치 환경)
- 2026-02-22: **WASM 빌드 검증** — emsdk 5.0.1로 빌드 성공
  - `libvoikko.wasm` 218KB + `libvoikko.mjs` 39KB
  - 37/37 테스트 통과 (Tier 1 + Tier 2 전체)
- 2026-02-22: **코드 리팩터링** — 구조 개선
  - wasm-loader: 단일 함수 → 3개 순수 함수 (`loadWasm`, `loadDict`, `mountDict`)
  - WASM 모듈 캐싱 (인스턴스 재생성 시 재로드 방지)
  - index.ts가 fork-join 파이프라인으로 오케스트레이션
  - 테스트: beforeAll/beforeEach 분리, 테스트 시간 210ms → 106ms
  - 사전 경로: flat 레이아웃 자동 감지, globalSetup으로 모노레포 사전 자동 감지

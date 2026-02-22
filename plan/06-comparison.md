# 종합 비교 및 추천 전략

## 4개 접근법 비교표

| 기준 | Emscripten | Rust + WASM | TypeScript | Zig + WASM |
|------|:----------:|:-----------:|:----------:|:----------:|
| **작업량** | **1-2주** | 16-23주 | 21-32주 | 24-34주 |
| **성능** (native 대비) | ~1.2-2x | ~1.0-1.2x | ~5-15x | ~1.0-1.2x |
| **번들 (WASM/코드)** | ~500KB | ~200-400KB | ~50KB | ~200-400KB |
| **번들 (사전 포함, gzip)** | ~1.7MB | ~1.7MB | ~1.7MB | ~1.7MB |
| **JS DX** | 보통 | **최상** | **최상** | 낮음 |
| **Tree-shaking** | 불가 | Feature flags | **완벽** | 불가 |
| **Upstream 추적** | **자동** | 불가 | 불가 | 불가 |
| **메모리 안전성** | 기존 수준 | **최상** | GC 의존 | 좋음 |
| **기존 바인딩 호환** | **유지** | 가능 | 별도 구현 | 가능 |
| **생태계 성숙도** | 매우 높음 | 높음 | 매우 높음 | 낮음 |
| **Node + Browser** | 지원 | 지원 | **어디서든** | 지원 |
| **정확성 리스크** | **없음** | 중간 | 높음 | 중간 |

## 점수 요약

| 접근법 | 점수 | 한줄 평가 |
|--------|:----:|----------|
| Emscripten | **8.4** | 이미 동작, 최소 리스크, 모던화만 필요 |
| Rust + WASM | **7.5** | 장기 최적, JS DX 최상, 투자 크지만 효과도 큼 |
| TypeScript | **5.5** | DX 최상이지만 성능/정확성/유지보수 리스크 |
| Zig + WASM | **5.5** | 기술적 매력 있으나 생태계 미성숙으로 시기상조 |

## 핵심 인사이트

### 사전 데이터가 지배적
모든 접근법에서 사전 데이터(~3.9 MB, gzip ~1.5 MB)가 번들의 80%+ 차지.
코드 크기 차이(50 KB vs 500 KB)는 전체 패키지 크기에서 미미.
-> Tree-shaking의 실질적 이점이 제한적.

### Emscripten 빌드가 이미 존재
`libvoikko/js/`에 완전한 빌드 파이프라인 존재. 이론이 아닌 실증.
C++ 소스 0줄 수정으로 동작하며 upstream 자동 추적.

### Rust는 장기 투자로 가치 있음
divvunspell (동일 도메인 Rust WASM 성공 선례) 존재.
메모리 안전성 + wasm-bindgen DX + 네이티브/WASM/C FFI 통합 빌드.

## 추천 전략: 2단계 접근

### Phase 1: Emscripten 모던화 (1-2주)

기존 `libvoikko/js/` 빌드를 현대적 npm 패키지로 탈바꿈.

**작업 목록:**
1. 최신 Emscripten 빌드 검증
2. ES Module 출력 (`-s EXPORT_ES6=1`)
3. TypeScript 선언 파일 (.d.ts)
4. Fetch 기반 사전 lazy loading
5. package.json + npm 배포 설정
6. CI/CD (GitHub Actions)
7. 테스트 현대화 (QUnit -> vitest)
8. 사용 문서

**성과물:**
- `npm install voikko` 한 방으로 핀란드어 NLP 사용 가능
- Node.js + Browser 모두 지원
- TypeScript 타입 포함

### Phase 2: Rust 포팅 (장기, 선택적)

Emscripten으로 JS 생태계 진입 후 장기적 모던화.

**단계별 접근:**
```
1단계: FST 엔진 + Spellchecker    (4-6주) -> MVP
2단계: Morphology 분석기           (3-4주)
3단계: Grammar + Hyphenation       (4-6주)
4단계: C FFI -> Python/Java 교체   (2-3주)
```

**Phase 2 시작 조건:**
- Phase 1 npm 패키지가 실제 사용자 확보
- 성능/번들 사이즈 개선 요구 발생
- 메모리 안전성이 이슈로 부상
- 기여자 중 Rust 역량 있는 인원 확보

### 제외된 접근법

**TypeScript 제외 이유:**
- 사전 데이터 지배로 tree-shaking 이점 미미
- 5-15x 성능 저하
- FinnishVfstAnalyzer 1,179줄 재작성의 높은 버그 리스크
- 영구적 이중 유지보수 부담

**Zig 제외 이유:**
- pre-1.0 언어에 프로덕션 의존 위험
- wasm-bindgen 없어 JS 바인딩 수동 작성
- 생태계/커뮤니티 미성숙
- Zig 1.0 (2026 후반) 이후 재평가 가능

## 타임라인

```
Week 1-2:   Phase 1 -- Emscripten 모던화 + npm 배포
Week 3-8:   사용자 피드백 수집, Phase 2 필요성 평가
Week 9+:    (선택) Phase 2 -- Rust 포팅 시작
```

## 조사 일자

- 2026-02-21: 4개 접근법 병렬 분석 완료

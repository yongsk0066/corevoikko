# Phase 2: Rust WASM 포팅 마스터 플랜

> 4개 리서치 문서(01~04)의 종합 결과를 바탕으로 한 세부 실행 계획.

## 기본 설정

- **Rust**: 1.93.1, edition 2024
- **개발 가이드라인**: [05-dev-guidelines.md](./05-dev-guidelines.md) 참조
  - 설계 철학 (Screaming Architecture, Parse Don't Validate, Pragmatic FP)
  - 코딩 원칙 (Trait 설계, 에러 처리, 소유권, 문자열 전략)
  - 검증 방법론 (Phase별 크로스체크, 차등 테스트 인프라)

## 목표

libvoikko C++ 라이브러리를 Rust로 재작성하여:
- WASM 번들 크기 200-400KB (gzip 80-150KB) 달성
- wasm-bindgen으로 TS 타입 자동 생성
- 기존 `Voikko` 클래스 API 100% 호환 (drop-in replacement)
- 메모리 안전성 (447개 수동 new/delete → 소유권 + Drop)

## 전체 구조

```
Phase 0: 프로젝트 셋업
Phase 1: FST 엔진 (핵심 기반)
Phase 2: 형태소 분석 + 맞춤법 검사 MVP
Phase 3: 추가 기능 (하이픈, 토크나이저, 제안)
Phase 4: 문법 검사 (가장 복잡)
Phase 5: WASM 통합 + TS 래퍼 교체
```

---

## Phase 0: 프로젝트 셋업

**의존성**: 없음 (즉시 시작 가능)

### 0-1. Cargo workspace 생성

```
libvoikko/rust/
├── Cargo.toml          # workspace root
├── crates/
│   ├── voikko-fst/     # FST 엔진 (Phase 1)
│   ├── voikko-core/    # 공유 타입 + character + utils (Phase 1과 병렬)
│   ├── voikko-fi/      # 핀란드어 모듈 (Phase 2-4)
│   └── voikko-wasm/    # wasm-bindgen 래퍼 (Phase 5)
├── test-data/          # 테스트용 .vfst 파일
└── build.sh            # cargo + wasm-bindgen-cli + wasm-opt 파이프라인
```

**crate 분리 근거**:
- `voikko-fst`: 언어 무관한 FST 엔진. 독립 테스트 가능.
- `voikko-core`: 공유 타입(Analysis, Token 등), 캐릭터 유틸리티. 모든 crate이 의존.
- `voikko-fi`: 핀란드어 특화 로직 (FinnishVfstAnalyzer, 문법 규칙 등).
- `voikko-wasm`: JS 바인딩만 담당. 가장 얇은 레이어.

### 0-2. Cargo.toml 설정

```toml
[workspace]
members = ["crates/*"]

[workspace.dependencies]
bytemuck = { version = "1", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
hashbrown = "0.15"
wasm-bindgen = "0.2"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

Feature flags:
- `spell` (default)
- `suggest` (depends on spell)
- `analyze`
- `hyphenate` (depends on analyze)
- `grammar` (depends on analyze)
- `tokenize`

### 0-3. 빌드 파이프라인

```bash
# Native 테스트
cargo test

# WASM 빌드
cargo build --target wasm32-unknown-unknown --release -p voikko-wasm
wasm-bindgen target/wasm32-unknown-unknown/release/voikko_wasm.wasm \
  --out-dir pkg --target bundler --typescript
wasm-opt pkg/voikko_wasm_bg.wasm -Oz -o pkg/voikko_wasm_bg.wasm
```

---

## Phase 1: FST 엔진

**의존성**: Phase 0 완료 후
**범위**: `voikko-fst` + `voikko-core` (병렬 진행 가능)
**추정 C++ 라인**: ~1,556 (fst/) + ~635 (character/) + ~602 (utils/)

### 1-A. voikko-core: 공유 타입 + 유틸리티 (Tier 0)

Phase 1-B와 **병렬** 진행 가능. 내부 의존성 없음.

| 작업 | C++ 원본 | Rust 대상 | 비고 |
|------|----------|-----------|------|
| 1-A-1. Enums | `voikko_enums.h`, `voikko_defines.h` | `voikko-core/src/enums.rs` | TokenType, SentenceType, spellresult, option constants |
| 1-A-2. Analysis | `morphology/Analysis.hpp` | `voikko-core/src/analysis.rs` | `HashMap<String, String>` 기반 |
| 1-A-3. Character | `character/SimpleChar.hpp`, `charset.hpp` | `voikko-core/src/character.rs` | Rust char + Unicode 크레이트 활용 |
| 1-A-4. Case utils | `utils/utils.hpp` | `voikko-core/src/case.rs` | casetype, voikko_casetype, voikko_set_case |
| 1-A-5. GrammarError | `grammar/VoikkoGrammarError.hpp` | `voikko-core/src/grammar_error.rs` | 공개 API 타입 |
| 1-A-6. Token/Sentence | `grammar/Token.hpp`, `Sentence.hpp` | `voikko-core/src/token.rs` | 공개 API 타입 |

**검증**: `cargo test -p voikko-core` — 순수 유닛 테스트

### 1-B. voikko-fst: FST 엔진 (Tier 1-2)

핵심 기반. 모든 상위 모듈이 의존.

| 작업 | C++ 원본 | Rust 대상 | 난이도 | 비고 |
|------|----------|-----------|--------|------|
| 1-B-1. Binary format | `Transducer.cpp:163-178` | `voikko-fst/src/format.rs` | 낮음 | 16B 헤더, 심볼 테이블, 패딩 |
| 1-B-2. Transitions | `Transition.hpp`, `WeightedTransition.hpp` | `voikko-fst/src/transition.rs` | 낮음 | `#[repr(C)]` + bytemuck zero-copy |
| 1-B-3. Symbol table | `UnweightedTransducer.cpp:125-189` | `voikko-fst/src/symbols.rs` | 낮음 | HashMap<char, u16>, Vec<String> |
| 1-B-4. Flag diacritics | `Transducer.cpp:62-123` | `voikko-fst/src/flags.rs` | 낮음 | 5개 연산 (P,C,U,R,D) |
| 1-B-5. Configuration | `Configuration.hpp/cpp` | `voikko-fst/src/config.rs` | 낮음 | Vec 기반 스택 |
| 1-B-6. Unweighted traversal | `UnweightedTransducer.cpp:228-370` | `voikko-fst/src/unweighted.rs` | **중간** | goto → labeled loop, 코루틴 패턴 |
| 1-B-7. Weighted traversal | `WeightedTransducer.cpp:230-428` | `voikko-fst/src/weighted.rs` | **중간** | Binary search, weight 누적, backtrack |

**의존성 순서**: 1→2→3+4 (병렬)→5→6+7 (병렬)

**간소화 결정**:
- ❌ Byte-swap 제거: WASM은 항상 LE, 사전도 LE
- ❌ mmap 제거: WASM에서는 `Vec<u8>`, native에서는 나중에 `memmap2` 추가 가능
- ✅ 통합 Config: u32 심볼 스택 사용 (unweighted도 u32로 upcast)
- ✅ Trait 기반: `Transducer` trait로 unweighted/weighted 통합 인터페이스

**검증**:
1. `cargo test -p voikko-fst` — 유닛 테스트 (수제 .vfst 파일)
2. 차등 테스트: voikkovfstc로 작은 ATT → .vfst 컴파일, C++/Rust 출력 비교

---

## Phase 2: 형태소 분석 + 맞춤법 MVP

**의존성**: Phase 1 완료 (FST 엔진 + 공유 타입)
**범위**: `voikko-fi` 핵심 모듈
**추정 C++ 라인**: ~2,525 (morphology/) + ~2,368 (spellchecker/ 일부)

### 2-A. 형태소 분석기

| 작업 | C++ 원본 | Rust 대상 | 난이도 | 비고 |
|------|----------|-----------|--------|------|
| 2-A-1. Analyzer trait | `Analyzer.hpp` | `voikko-fi/src/morphology/mod.rs` | 낮음 | trait Analyzer { fn analyze(...) } |
| 2-A-2. VfstAnalyzer | `VfstAnalyzer.cpp` (~120줄) | `voikko-fi/src/morphology/vfst.rs` | 낮음 | Generic weighted FST 분석 |
| 2-A-3. FinnishVfstAnalyzer | `FinnishVfstAnalyzer.cpp` (~1,179줄) | `voikko-fi/src/morphology/finnish.rs` | **높음** | 태그 파싱, 복합어 검증, 기본형 도출 |

⚠️ **2-A-3이 최대 리스크**: off-by-one 에러가 모든 하위 모듈에 전파됨.

**리스크 완화**:
- FST 출력 파서를 독립 함수로 분리하여 단위 테스트
- C++ 빌드에 계측 코드 삽입 → (입력, FST출력, Analysis) 트리플 캡처
- 캡처한 트리플로 골든파일 테스트 구성

### 2-B. 맞춤법 검사

| 작업 | C++ 원본 | Rust 대상 | 난이도 | 비고 |
|------|----------|-----------|--------|------|
| 2-B-1. Speller trait | `Speller.hpp` | `voikko-fi/src/speller/mod.rs` | 낮음 | trait Speller { fn spell(...) } |
| 2-B-2. SpellUtils | `SpellUtils.cpp` | `voikko-fi/src/speller/utils.rs` | 낮음 | STRUCTURE 매칭 |
| 2-B-3. AnalyzerToSpellerAdapter | `AnalyzerToSpellerAdapter.cpp` | `voikko-fi/src/speller/adapter.rs` | 낮음-중간 | Analyzer → Speller 브릿지 |
| 2-B-4. SpellerCache | `SpellerCache.cpp` | `voikko-fi/src/speller/cache.rs` | 낮음 | 해시 기반 캐시 |
| 2-B-5. FinnishSpellerTweaks | `FinnishSpellerTweaksWrapper.cpp` | `voikko-fi/src/speller/finnish.rs` | 중간 | 소프트 하이픈, 옵셔널 하이픈 |
| 2-B-6. Spell pipeline | `spell.cpp` | `voikko-fi/src/speller/pipeline.rs` | 중간 | 정규화, 케이스 처리, 캐시 |

**의존성 순서**: 1→2→3→4→5→6 (순차)
단, 2-B-1~4는 2-A-2 완료 후 시작 가능 (FinnishVfstAnalyzer 불필요)

### Phase 2 검증

```
✅ cargo test — FinnishVfstAnalyzer 유닛 테스트 (골든파일)
✅ cargo test — spell("koira") == true, spell("koirra") == false
✅ 차등 테스트 — 10,000단어 리스트로 C++ vs Rust 비교
```

**마일스톤**: `voikko.spell("koira")` 가 Rust native에서 동작

---

## Phase 3: 추가 기능

**의존성**: Phase 2 완료
**병렬 가능**: 3-A, 3-B, 3-C는 서로 독립적 → 동시 진행 가능

### 3-A. 하이픈 처리

| 작업 | 난이도 | 비고 |
|------|--------|------|
| 3-A-1. Hyphenator trait | 낮음 | |
| 3-A-2. FinnishHyphenator | **높음** | 핀란드어 음운 규칙, 복합어 구조 |

~540줄. 핀란드어 모음/자음 테이블을 static const로 추출.

### 3-B. 토크나이저 + 문장 분리

| 작업 | 난이도 | 비고 |
|------|--------|------|
| 3-B-1. Tokenizer | 중간 | URL/이메일 감지, Unicode 구두점 |
| 3-B-2. Sentence detector | 낮음 | |

~376줄. 비교적 단순하고 언어 무관.

### 3-C. 제안 생성

| 작업 | 난이도 | 비고 |
|------|--------|------|
| 3-C-1. SuggestionGenerator trait | 낮음 | |
| 3-C-2. SuggestionStatus + 우선순위 | 낮음 | abort/cost 로직 |
| 3-C-3. 12개 개별 Generator | 중간 (각각 낮음) | 병렬 구현 가능 |
| 3-C-4. SuggestionStrategy (typing/ocr) | 중간 | 오케스트레이션 |
| 3-C-5. VfstSuggestion | **중간-높음** | 2개 트랜스듀서 조합 |

~2,615줄 (suggestion/ 전체). 12개 Generator는 각각 독립적이라 **병렬 구현에 적합**.

---

## Phase 4: 문법 검사

**의존성**: Phase 3-B (토크나이저) 완료 필수, Phase 2 완료 필수
**범위**: ~3,395줄 (grammar/ 전체)

| 작업 | 난이도 | 비고 |
|------|--------|------|
| 4-1. Token/Sentence/Paragraph 구조 | 낮음 | 이미 voikko-core에 정의 |
| 4-2. GcCache | 낮음-중간 | 해시 기반 문단 캐시 |
| 4-3. FinnishAnalysis (토큰 주석) | **중간-높음** | 형태소 분석 → 토큰 플래그 매핑 |
| 4-4. CapitalizationCheck | 중간 | 5-state FSA |
| 4-5. gc_local_punctuation 등 | 낮음 (각각) | 6개 문장 레벨 체크 |
| 4-6. MissingVerbCheck 등 | 중간 (각각) | 4개 핀란드어 동사 규칙 |
| 4-7. VfstAutocorrectCheck | 중간 | autocorr.vfst 트랜스듀서 사용 |
| 4-8. FinnishRuleEngine 조합 | 낮음 | 위 체크들 오케스트레이션 |
| 4-9. GrammarChecker | 낮음 | trait 구현 |

**병렬 가능**: 4-5와 4-6의 개별 체크들은 서로 독립적.

---

## Phase 5: WASM 통합 + TS 래퍼 교체

**의존성**: Phase 4 완료 (전체 기능 구현)

### 5-1. voikko-wasm crate

```rust
#[wasm_bindgen]
pub struct WasmVoikko { handle: VoikkoHandle }

#[wasm_bindgen]
impl WasmVoikko {
    #[wasm_bindgen(constructor)]
    pub fn new(dict_data: &[u8], lang: &str) -> Result<WasmVoikko, JsError> { ... }
    pub fn spell(&self, word: &str) -> bool { ... }
    pub fn suggest(&self, word: &str) -> Vec<String> { ... }
    pub fn analyze(&self, word: &str) -> JsValue { ... }  // serde-wasm-bindgen
    // ... (15개 메서드 + 14개 옵션 setter)
    pub fn terminate(&mut self) { ... }
}
```

### 5-2. TS 래퍼 업데이트

- `wasm-loader.ts` 수정: Emscripten → wasm-bindgen 모듈 로드
- `Voikko` 클래스 API는 그대로 유지
- 사전: 파일별 fetch → Uint8Array → Rust 생성자 전달 (VFS 불필요)
- FinalizationRegistry로 자동 cleanup

### 5-3. 빌드 파이프라인 통합

- `pnpm build:wasm` 스크립트 업데이트
- CI에서 Rust WASM 빌드 + vitest 실행

### 5-4. 차등 테스트 + 회귀 검증

- 기존 vitest 37개 테스트 전부 통과 확인
- 10,000+ 단어 리스트로 C++ vs Rust 전수 비교

---

## 의존성 다이어그램

```
Phase 0: 셋업
    │
    ├── Phase 1-A: voikko-core ──────────┐
    │   (enums, Analysis, character,     │
    │    case, Token, GrammarError)       │
    │                                     │
    ├── Phase 1-B: voikko-fst ───────────┤
    │   (format, transitions, symbols,   │
    │    flags, config, traversal)        │
    │                                     │
    ▼                                     ▼
Phase 2-A: 형태소 분석 ─────┬── Phase 2-B: 맞춤법 검사
    │                       │        │
    │  ┌────────────────────┘        │
    │  │                             │
    ▼  ▼                             ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│Phase 3-A │  │Phase 3-B │  │Phase 3-C │
│하이픈    │  │토크나이저│  │제안 생성 │
└──────────┘  └────┬─────┘  └──────────┘
                   │
                   ▼
           Phase 4: 문법 검사
                   │
                   ▼
           Phase 5: WASM 통합
```

## 작업 패러다임: 에이전트 병렬화

### 병렬 가능한 작업 그룹

| 그룹 | 작업들 | 이유 |
|------|--------|------|
| **Phase 1** | 1-A 전체 ∥ 1-B-1~4 | voikko-core와 voikko-fst의 기초는 독립적 |
| **Phase 1** | 1-B-6 ∥ 1-B-7 | Unweighted와 Weighted 순회는 별개 구현 |
| **Phase 3** | 3-A ∥ 3-B ∥ 3-C | 하이픈, 토크나이저, 제안은 서로 독립 |
| **Phase 3-C** | 12개 SuggestionGenerator | 각 Generator는 독립 구현 |
| **Phase 4** | 4-5 ∥ 4-6 | 개별 문법 체크들은 서로 독립 |

### 순차 필수 작업

| 선행 | 후행 | 이유 |
|------|------|------|
| Phase 1 | Phase 2 | FST 엔진이 형태소 분석의 기반 |
| Phase 2-A | Phase 2-B | 맞춤법 = 형태소 분석 기반 |
| Phase 2 | Phase 3-A | 하이픈은 형태소 분석 결과 필요 |
| Phase 2 | Phase 3-C | 제안 생성에 Speller 필요 |
| Phase 3-B | Phase 4 | 문법 검사에 토크나이저 필요 |
| Phase 4 | Phase 5 | 전체 기능 구현 후 WASM 통합 |

---

## 리스크 매트릭스

| 리스크 | 확률 | 영향 | 완화 |
|--------|------|------|------|
| FinnishVfstAnalyzer FST 태그 파싱 오류 | 높음 | 높음 | 골든파일 테스트, C++ 계측 빌드 |
| wchar_t → String 인덱싱 버그 | 중간 | 높음 | Vec<char> 사용, 차등 테스트 |
| 핀란드어 음운 규칙 오류 (하이픈) | 중간 | 중간 | 전체 사전 단어로 차등 테스트 |
| 제안 품질 저하 (cost model 차이) | 낮음 | 중간 | 정확한 abort/priority 로직 복제 |
| WASM 번들 크기 초과 | 낮음 | 낮음 | Feature flags, wasm-opt, twiggy |
| wasm-bindgen 호환성 문제 | 낮음 | 낮음 | 버전 고정, 릴리스 모니터링 |

---

## MVP 타임라인 (Phase 0-2)

Phase 0-2까지 완료하면 `voikko.spell()` 이 Rust WASM에서 동작하는 MVP가 된다.

```
Phase 0 (셋업):          ~1일
Phase 1-A (voikko-core): ~2-3일  ┐
Phase 1-B (voikko-fst):  ~5-7일  ┘ 병렬 → 총 5-7일
Phase 2-A (형태소):      ~7-10일
Phase 2-B (맞춤법):      ~3-5일
                         ─────────
MVP 합계:               ~16-23일
```

## 리서치 문서 참조

| 문서 | 내용 |
|------|------|
| [01-dependency-graph.md](./01-dependency-graph.md) | C++ 모듈 의존성 그래프, 공유 타입, 호출 흐름 |
| [02-fst-engine.md](./02-fst-engine.md) | VFST 바이너리 포맷, 순회 알고리즘, Rust 타입 매핑 |
| [03-modules-analysis.md](./03-modules-analysis.md) | 맞춤법/형태소/문법/하이픈/토크나이저 상세 분석 |
| [04-rust-ecosystem.md](./04-rust-ecosystem.md) | divvunspell, wasm-bindgen, 빌드 도구, 테스팅 전략 |
| [05-dev-guidelines.md](./05-dev-guidelines.md) | 개발 철학, 코딩 원칙, 검증 방법론 |

---

## 진행 일지

- 2026-02-22: Phase 2 리서치 완료 — 4개 병렬 에이전트로 C++ 코드베이스 심층 분석
  - 01-dependency-graph.md: 5개 의존성 티어, 11개 Tier 0 독립 모듈 식별
  - 02-fst-engine.md: VFST 바이너리 포맷 바이트 레벨 스펙, ~835줄 Rust 추정
  - 03-modules-analysis.md: FinnishVfstAnalyzer 1,179줄 상세 분석, 18개 문법 에러 코드
  - 04-rust-ecosystem.md: divvunspell 아키텍처, wasm-pack 아카이브 확인, 수동 빌드 파이프라인 권장
- 2026-02-22: 마스터 플랜 작성 (00-master-plan.md)
- 2026-02-22: 개발 가이드라인 작성 (05-dev-guidelines.md) — Rust 1.93.1, 설계 철학, 검증 방법론
- 2026-02-22: Phase 0 완료 — Cargo workspace 생성, 4 crate 스캐폴딩, CLAUDE.md per crate
- 2026-02-22: Phase 1 완료 — voikko-core + voikko-fst 병렬 구현
  - voikko-core: 6 모듈 (enums, analysis, character, case, grammar_error, token) — 66 tests, clippy 0
  - voikko-fst: 7 모듈 (format, transition, symbols, flags, config, unweighted, weighted) — 71 tests, clippy 0
  - 전체 workspace: 137 tests 통과, clippy 경고 0건
- 2026-02-22: Phase 2 완료 — 형태소 분석 + 맞춤법 검사 MVP (병렬 구현)
  - Phase 2-A 형태소 분석: tag_parser (순수 함수 13개 lookup + 6개 파서), FinnishVfstAnalyzer, VfstAnalyzer — 35 tests
  - Phase 2-B 맞춤법 검사: Speller trait, STRUCTURE 매칭, AnalyzerToSpellerAdapter, SpellerCache, FinnishSpellerTweaks, pipeline — 80 tests
  - 전체 workspace: 252 tests 통과, clippy 경고 0건
- 2026-02-22: Phase 2 품질 검증 (82/100) — Critical 2건 수정 (C-1 RefCell, C-2 position-aware scan), Warning 3건 수정
- 2026-02-22: Phase 3 완료 — 하이픈 + 토크나이저 + 제안 (3팀 병렬 구현)
  - Phase 3-A 하이픈: FinnishHyphenator (7개 음절 규칙, 복합어 경계, SPLIT_VOWELS/LONG_CONSONANTS) — 47 tests (but feature-gated behind hyphenate, counted in voikko-fi total)
  - Phase 3-B 토크나이저: next_token, URL/email 감지, next_sentence — 88 tests
  - Phase 3-C 제안: 12개 SuggestionGenerator, SuggestionStatus, typing/OCR strategy — 37 tests (VfstSuggestion은 TODO)
  - 전체 workspace: 340 tests 통과, clippy 경고 0건
- 2026-02-22: Phase 3 품질 검증 (82/100) — Critical 1건 수정 (C-1 hyphen backward scan), Warning 1건 수정 (W-1 double analysis)

- 2026-02-22: Phase 4 완료 — 문법 검사 + VfstSuggestion (3팀 병렬 구현)
  - Phase 4-A 기반: paragraph, cache, FinnishAnalysis — 43 tests
  - Phase 4-B 체크 규칙: 9개 check + CapitalizationCheck(5-state FSA) + autocorrect + engine + checker — 59 tests
  - VfstSuggestion: dual weighted transducer 제안 생성기 — 7 tests (Phase 3 TODO 해결)
  - 전체 workspace: 549 tests (all-features), clippy 경고 0건

- 2026-02-22: Phase 4 품질 검증 (82/100) — Critical 1건 수정 (GcCache RefCell), Warning 1건 수정 (i16→i32 overflow)

- 2026-02-22: Phase 5 완료 — WASM 통합 + 리팩터링 + TODO 해결 (4팀 병렬 구현)
  - VoikkoHandle (handle.rs): 전체 통합 — spell, suggest, analyze, hyphenate, grammar_errors, tokens, sentences + 16 옵션 setter
  - WasmVoikko (voikko-wasm/src/lib.rs): wasm-bindgen 바인딩 — DTO 타입, serde-wasm-bindgen 직렬화, camelCase JS 명명
  - 리팩터링: finnish/constants.rs (상수 통합), dead_code 정리 (module-level), autocorrect 12 tests (VFST 빌더)
  - Feature TODO 해결:
    - checker → analyse_paragraph: analyzer 파라미터 추가, check_with_analyzer 메서드
    - suggest CapitalizationError: STRUCTURE 기반 대소문자 보정 (apply_structure_case)
    - priority_from_result: word class + inflection + structure 기반 우선순위
    - Sentence 약어 감지: next_sentence_with_speller 추가
  - C++ 빌드 수정: make clean → 링커 에러 해결, voikkospell 동작 확인
  - 차등 테스트 인프라: golden file 생성 (246단어 spell/analyze/hyphenate/suggest)
  - 전체 workspace: 587 tests (all-features), clippy 경고 0건

- 2026-02-22: Phase 5 품질 검증 (85/100) — Critical 2건 수정 (unwrap→zip, grammar analyzer 연결), Warning 3건 수정

- 2026-02-22: Suggest rich priority 구현 완료
  - SuggestionGenerator trait에 `analyzer: Option<&dyn Analyzer>` 파라미터 추가
  - 12개 generator + SplitWord::spell_ok에 compute_priority 적용
  - SuggestionStrategy::generate() 경로 연결
  - 차등 테스트: suggest 6/31 (81%) → 31/31 (100%)
  - spell/analyze/hyphenate 100% 유지, clippy 0건
  - 전체 workspace: 625 tests (all-features), 3 ignored

### 미해결 TODO / 향후 작업

| 항목 | 원인 | 해결 시점 |
|------|------|-----------|
| ~~**VfstSuggestion**~~ | ✅ 구현 완료 | Phase 4에서 해결 |
| ~~**suggest_for_buffer CapitalizationError**~~ | ✅ STRUCTURE 기반 대소문자 보정 구현 | Phase 5에서 해결 |
| ~~**priority_from_result**~~ | ✅ word class/inflection/compound 기반 우선순위 구현 | Phase 5에서 해결 |
| ~~**Sentence 약어 감지**~~ | ✅ next_sentence_with_speller 구현 | Phase 5에서 해결 |
| ~~**핀란드어 상수 통합**~~ | ✅ finnish/constants.rs 생성 | Phase 5에서 해결 |
| ~~**checker tokenize 중복**~~ | ✅ check_with_analyzer 메서드로 외부 analyzer 전달 | Phase 5에서 해결 |
| ~~**dead_code allow 정리**~~ | ✅ module-level로 이동 | Phase 5에서 해결 |
| ~~**autocorrect 테스트 보강**~~ | ✅ 12 tests (VFST 빌더 포함) | Phase 5에서 해결 |
| ~~**SpellerCache RefCell**~~ | ✅ RefCell 래핑, spell()에서 캐시 사용 | TODO 해결 시 |
| ~~**Rust 차등 테스트 러너**~~ | ✅ tests/differential.rs 작성 — spell/analyze/hyphenate/suggest 비교 | TODO 해결 시 |
| ~~**WASM 빌드 검증**~~ | ✅ 197KB .wasm + .d.ts 생성 확인 | TODO 해결 시 |
| ~~**soft hyphen 검증**~~ | ✅ hyphenator로 유효 위치 검증 | TODO 해결 시 |
| ~~**Unicode normalize**~~ | ✅ C++ voikko_normalise 완전 구현 (67 combining mark) | TODO 해결 시 |
| ~~**STRUCTURE off-by-one**~~ | ✅ [Bc] 태그 i+=4→i+=3 수정 | 차등 테스트에서 발견 |
| ~~**suggest 3x 슬롯**~~ | ✅ max_suggestions*3 후보 수집 후 truncate | 차등 테스트에서 발견 |
| ~~**suggest rich priority**~~ | ✅ SuggestionGenerator trait에 analyzer 파라미터 추가, 12개 generator 수정 → 차등 테스트 31/31 (100%) | 구현 완료 |
| **TS 래퍼 연결** | libvoikko/js/ ESM 래퍼가 Rust WASM 백엔드 사용하도록 전환 | TS 통합 시 |

---

## 실제 사전 검증 결과 (2026-02-22)

### FST 엔진 + 형태소 분석 파이프라인 검증

`voikko-fi/vvfst/mor.vfst` (3.8MB, 289 symbols, 23 flag features)를 Rust FST 엔진으로 로드하여
실제 핀란드어 단어를 분석한 결과 전체 파이프라인이 정상 동작 확인됨.

| 입력 | class | baseform | case | number | STRUCTURE |
|------|-------|----------|------|--------|-----------|
| koira | nimisana | koira | nimento | singular | =ppppp |
| Helsinki | paikannimi | Helsinki | nimento | singular | =ippppppp |
| juoksen | teonsana | juosta | - | singular | =ppppppp |
| koiran | nimisana | koira | omanto | singular | =pppppp |
| kissalle | nimisana | kissa | ulkotulento | singular | =pppppppp |
| koiranruoka | nimisana | koiranruoka | nimento | singular | =pppppp=ppppp |
| suurempi | laatusana | suuri | nimento | singular | =pppppppp |
| asdfxyz | (없음) | - | - | - | - |

검증 도구: `crates/voikko-fst/examples/fst_test.rs`, `crates/voikko-fi/examples/analyze_test.rs`

### C++ 차등 테스트 (golden file 생성 완료)

C++ 빌드 수정 완료 (`make clean && make`). 원인: 이전 빌드 중단으로 `.lo` 파일 PIC 상태 불일치.
voikkospell 동작 확인: `koira/kissa/Helsinki/juoksen` → C, `koirra/asdfxyz` → W.

**차등 테스트 인프라:**

```
libvoikko/rust/tests/differential/
├── wordlist.txt           # 246단어 테스트 리스트
├── capture_cpp.py         # C++ libvoikko Python 바인딩으로 출력 캡처
└── golden/
    ├── spell.json         # 213/246 correct
    ├── analyze.json       # 88KB, 213단어 분석 결과
    ├── hyphenate.json     # 6.6KB, 하이픈 패턴
    └── suggest.json       # 31개 오탈자 제안
```

**차등 테스트 실행 절차:**
1. C++ 빌드: `cd libvoikko && make clean && make`
2. golden file 재생성: `DYLD_LIBRARY_PATH=src/.libs python3 capture_cpp.py`
3. Rust 비교: (러너 작성 필요)

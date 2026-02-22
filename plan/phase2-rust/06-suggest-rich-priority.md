# Rich Priority 제안 시스템 구현 계획

> SuggestionGenerator trait에 Analyzer를 전달하여 C++과 동일한 형태소 분석 기반 우선순위 계산을 구현한다.

## 1. 배경 및 동기

### 1.1 현재 상태

차등 테스트 결과 suggest에서 **6/31 (19%)** 불일치가 발생한다.
spell(100%), analyze(100%), hyphenate(100%)는 완벽히 일치.

### 1.2 불일치 원인 분석

C++과 Rust의 제안 시스템에 두 가지 설계 차이가 있다:

1. **✅ 해결됨 — 슬롯 수**: C++은 `max_suggestions * 3 = 15`개 후보 수집 → 정렬 → 상위 5개 반환.
   Rust도 동일하게 수정 완료 (commit `bec7f04a`).

2. **⬜ 미해결 — 우선순위 계산**: C++의 모든 generator는 `morAnalyzer`를 받아 형태소 분석 기반
   rich priority를 계산한다. Rust는 `suggest_for_buffer(speller, ...)` 로만 호출하여
   flat priority (Ok=1, CapFirst=2, CapError=3)를 사용한다.

### 1.3 C++ 아키텍처

```
                          SuggestionStrategy
                          ├── CaseChange(morAnalyzer)
                          ├── VowelChange(morAnalyzer)
                          ├── Replacement(morAnalyzer)
                          ├── Deletion(morAnalyzer)
                          ├── ...
                          └── 각 generator가 suggestForBuffer(morAnalyzer, ...) 호출
                                      ↓
                              SpellWithPriority::spellWithPriority(morAnalyzer, word, len, &prio)
                                      ↓
                              morAnalyzer->analyze(word) → Analysis[]
                                      ↓
                              getPriorityFromWordClassAndInflection() → class_prio
                              getPriorityFromStructure() → struct_prio (compound penalty)
                              result_prio (Ok=2, CapFirst=2, CapError=4)
                              priority = class_prio * struct_prio * result_prio
```

모든 C++ generator 클래스는 생성자에서 `Analyzer * morAnalyzer`를 받고 저장한다:
- `SuggestionGeneratorCaseChange(morAnalyzer)`
- `SuggestionGeneratorDeletion(morAnalyzer)`
- `SuggestionGeneratorInsertion(morAnalyzer)`
- `SuggestionGeneratorReplacement(morAnalyzer)`
- `SuggestionGeneratorSwap(morAnalyzer)`
- `SuggestionGeneratorVowelChange(morAnalyzer)`
- `SuggestionGeneratorSplitWord(morAnalyzer)`
- `SuggestionGeneratorInsertSpecial(morAnalyzer)`
- `SuggestionGeneratorSoftHyphens(morAnalyzer)`
- `SuggestionGeneratorReplaceTwo(morAnalyzer)`
- `SuggestionGeneratorDeleteTwo(morAnalyzer)`
- `SuggestionGeneratorMultiReplacement(morAnalyzer)`

C++ 인터페이스: `void generate(SuggestionStatus * s) const;` — morAnalyzer는 **필드**에 저장.

### 1.4 Rust 현재 아키텍처

```rust
pub trait SuggestionGenerator {
    fn generate(&self, speller: &dyn Speller, status: &mut SuggestionStatus<'_>);
}
```

- `speller`만 받고 `analyzer` 없음
- 각 generator는 `suggest_for_buffer(speller, status, buffer, len)` 호출
- `suggest_for_buffer`는 `suggest_for_buffer_with_analyzer(..., None)` 위임
- `None`이므로 항상 flat priority (`priority_from_result`)

### 1.5 이미 구현된 인프라

generators.rs에 rich priority 함수들이 **이미 구현**되어 있다:

```rust
fn priority_from_noun_inflection(sijamuoto: &str) -> i32           // ✅ 구현됨
fn priority_from_word_class_and_inflection(class: &str, ...) -> i32 // ✅ 구현됨
fn priority_from_structure(structure: &str) -> i32                  // ✅ 구현됨
fn priority_from_analysis(analysis: &Analysis, result: SpellResult) -> i32 // ✅ 구현됨
fn best_priority_from_analyses(analyses: &[Analysis], result: SpellResult) -> i32 // ✅ 구현됨
fn compute_priority(analyzer: Option<&dyn Analyzer>, ...) -> i32    // ✅ 구현됨
fn suggest_for_buffer_with_analyzer(speller, status, buf, len, analyzer: Option<&dyn Analyzer>) // ✅ 구현됨
```

**유일한 문제: generator들이 `suggest_for_buffer_with_analyzer`를 `analyzer = None`으로 호출한다.**

---

## 2. 구현 계획

### 2.1 접근 방식: SuggestionGenerator trait에 analyzer 추가

두 가지 옵션이 있다:

#### Option A: trait 시그니처 변경 (권장)

```rust
pub trait SuggestionGenerator {
    fn generate(
        &self,
        speller: &dyn Speller,
        analyzer: Option<&dyn Analyzer>,  // 추가
        status: &mut SuggestionStatus<'_>,
    );
}
```

**장점**: 간단, 명확, C++ 패턴과 일치
**단점**: trait 시그니처 변경 → 모든 구현체 + 호출부 수정 필요

#### Option B: generator 구조체에 analyzer 저장 (C++ 패턴 그대로)

```rust
pub struct Deletion {
    analyzer: Option<Rc<dyn Analyzer>>,  // 또는 &'a dyn Analyzer
}
```

**장점**: C++과 1:1 매핑
**단점**: Rc 필요, 12개 struct 수정, 라이프타임 복잡

#### 결정: **Option A 채택**

이유:
- Rust에서는 trait 파라미터가 더 관용적 (필드에 analyzer 저장하면 라이프타임 문제)
- 변경 범위가 예측 가능 (trait + 12 구현체 + 2 호출부)
- `suggest_for_buffer_with_analyzer`가 이미 `Option<&dyn Analyzer>` 시그니처를 가짐

### 2.2 수정 대상 파일

| 파일 | 변경 내용 |
|------|----------|
| `suggestion/generators.rs` | trait 시그니처 + 12개 구현체의 generate() 파라미터 추가 |
| `suggestion/strategy.rs` | `SuggestionStrategy::generate()` 파라미터 추가, factory 함수 수정 |
| `suggestion/status.rs` | 변경 없음 |
| `handle.rs` | `suggest()` 메서드에서 analyzer 전달 |

### 2.3 상세 변경 사항

#### Step 1: trait 시그니처 변경

```rust
// generators.rs
pub trait SuggestionGenerator {
    fn generate(
        &self,
        speller: &dyn Speller,
        analyzer: Option<&dyn Analyzer>,
        status: &mut SuggestionStatus<'_>,
    );
}
```

#### Step 2: 12개 generator 구현체 수정

각 generator의 `generate()` 메서드에서:

**Before:**
```rust
impl SuggestionGenerator for Deletion {
    fn generate(&self, speller: &dyn Speller, status: &mut SuggestionStatus<'_>) {
        // ... 내부에서
        suggest_for_buffer(speller, status, &buffer, buf_len);
    }
}
```

**After:**
```rust
impl SuggestionGenerator for Deletion {
    fn generate(&self, speller: &dyn Speller, analyzer: Option<&dyn Analyzer>, status: &mut SuggestionStatus<'_>) {
        // ... 내부에서
        suggest_for_buffer_with_analyzer(speller, status, &buffer, buf_len, analyzer);
    }
}
```

변경할 12개 generator:
1. `CaseChange` — `suggest_for_buffer` → `suggest_for_buffer_with_analyzer` (이미 analyzer 로직 포함)
2. `SoftHyphens` — 동일 패턴
3. `VowelChange` — 동일 패턴
4. `Replacement` — 동일 패턴
5. `Deletion` — 동일 패턴
6. `InsertSpecial` — 동일 패턴
7. `SplitWord` — `priority_from_result` → `compute_priority` (이미 구현됨, analyzer 전달만 필요)
8. `ReplaceTwo` — 동일 패턴
9. `Insertion` — 동일 패턴
10. `Swap` — 동일 패턴
11. `DeleteTwo` — 동일 패턴
12. `MultiReplacement` — 동일 패턴

#### Step 3: SuggestionStrategy 수정

```rust
// strategy.rs
impl SuggestionStrategy {
    pub fn generate(
        &self,
        speller: &dyn Speller,
        analyzer: Option<&dyn Analyzer>,  // 추가
        status: &mut SuggestionStatus<'_>,
    ) {
        status.set_max_cost(self.max_cost);
        for generator in &self.primary_generators {
            if status.should_abort() { break; }
            generator.generate(speller, analyzer, status);  // analyzer 전달
        }
        if status.suggestion_count() > 0 { return; }
        for generator in &self.generators {
            if status.should_abort() { break; }
            generator.generate(speller, analyzer, status);  // analyzer 전달
        }
    }
}
```

#### Step 4: handle.rs 수정

```rust
// handle.rs — suggest() 메서드
pub fn suggest(&self, word: &str) -> Vec<String> {
    // ... (기존 speller 생성 코드)
    let mut status = SuggestionStatus::new(&word_chars, self.max_suggestions * 3);
    let strategy = if self.use_ocr_suggestions {
        &self.ocr_strategy
    } else {
        &self.typing_strategy
    };
    strategy.generate(&tweaks, Some(&self.analyzer), &mut status);  // analyzer 전달
    status.sort_suggestions();
    status.into_suggestions().into_iter().take(self.max_suggestions).map(|s| s.word).collect()
}
```

#### Step 5: suggest_for_buffer 정리

현재 `suggest_for_buffer`는 backward-compat wrapper:
```rust
pub fn suggest_for_buffer(speller, status, buffer, buf_len) {
    suggest_for_buffer_with_analyzer(speller, status, buffer, buf_len, None);
}
```

이 함수는 더 이상 외부에서 직접 호출되지 않으므로 **삭제 가능**.
generator들이 모두 `suggest_for_buffer_with_analyzer`를 직접 호출하게 된다.

단, `suggest_for_buffer`는 테스트에서도 사용될 수 있으므로 테스트에서 사용 중인지 확인 후 결정.

---

## 3. 검증 방법

### 3.1 기존 테스트 통과

```bash
cargo test --all-features
# 625+ tests 전부 통과해야 함
```

### 3.2 Clippy

```bash
cargo clippy --all-features -- -D warnings
# 경고 0건
```

### 3.3 차등 테스트 개선 확인

```bash
VOIKKO_DICT_PATH=/path/to/vvfst cargo test -p voikko-fi --features handle --test differential differential_suggest -- --nocapture
```

**기대 결과**: 6/31 불일치 → 0~2/31로 감소.

완전 0이 되지 않을 수 있는 이유:
- Rust의 HashSet 기반 중복 제거 vs C++의 중복 허용
- i32 saturating arithmetic vs C++ wrapping
- 부동소수점 없지만 정수 곱셈 순서가 다를 수 있음

### 3.4 단어별 검증

불일치가 남는 경우, 각 단어에 대해:
```bash
# C++ 제안
echo "autoo" | DYLD_LIBRARY_PATH=libvoikko/src/.libs libvoikko/src/tools/.libs/voikkospell -p voikko-fi/vvfst/ -s
```

와 Rust 결과를 비교하여 원인 분류:
- **ORDER_DIFF**: 같은 제안, 다른 순서 → 허용 (우선순위 미세 차이)
- **CONTENT_DIFF**: 다른 제안 → 조사 필요

### 3.5 성능 회귀 확인

rich priority는 각 제안 후보마다 `analyzer.analyze()` 를 추가 호출한다.
이는 추가 FST 순회를 의미하므로 성능 영향이 있을 수 있다.

```bash
# 벤치마크 (나중에)
time echo "koirra" | voikkospell -s  # C++ 기준선
# Rust 기준선은 criterion 벤치마크로 측정
```

C++도 동일하게 매번 analyze를 호출하므로, 성능이 비슷해야 한다.
다만 Rust의 `compute_priority` 호출 경로를 확인:
- `suggest_for_buffer_with_analyzer` → `compute_priority` → `analyzer.analyze()`
- 이 호출이 `SpellResult::Ok`와 `CapitalizeFirst` 에서만 발생하므로,
  `Failed` 결과에는 분석 호출 없음 → 성능 영향 최소화

---

## 4. 엣지 케이스 및 주의사항

### 4.1 SplitWord의 특수 처리

C++ `SuggestionGeneratorSplitWord`에서 `spellOk`는 내부적으로 `SpellWithPriority::spellWithPriority`를
호출하여 각 파트의 우선순위를 계산한다. Rust의 `SplitWord`는 현재:

```rust
fn spell_ok(speller: &dyn Speller, status: &mut SuggestionStatus, word: &mut [char]) -> (bool, i32) {
    let result = speller.spell(word, word.len());
    status.charge();
    (result != SpellResult::Failed, priority_from_result(result))
}
```

이것을 `compute_priority(analyzer, ...)` 로 변경해야 한다:

```rust
fn spell_ok(
    speller: &dyn Speller,
    analyzer: Option<&dyn Analyzer>,
    status: &mut SuggestionStatus,
    word: &mut [char],
) -> (bool, i32) {
    let result = speller.spell(word, word.len());
    status.charge();
    (result != SpellResult::Failed, compute_priority(analyzer, word, word.len(), result))
}
```

### 4.2 VfstSuggestion

`VfstSuggestion`은 `SuggestionGenerator` trait을 구현하지만, 자체 weighted transducer를 사용한다.
이 generator는 `suggest_for_buffer`를 호출하지 않고 직접 `status.add_suggestion()`을 호출한다.
따라서 analyzer 파라미터는 받지만 **사용하지 않아도 무방**하다.

### 4.3 case 4 (paalta → päältä) 퇴화 가능성

분석 결과 case 4에서 Rust가 C++보다 **더 나은** 결과를 보였다:
- Rust: "päältä" (올바른 핀란드어 단어) 1위
- C++: "päältä" 누락 (rich priority가 ablative 격변화에 높은 페널티)

rich priority 적용 후, "päältä"의 priority가 `noun_inflection("ulkoeronto") = 30`이 되어
`30 * 1 * 2 = 60`이 될 수 있다. 반면 "paalata" (verb, nominative)는 `4 * 1 * 2 = 8`로
더 좋은(낮은) priority를 받는다.

이 경우 C++과 동일하게 "päältä"가 top 5에서 밀려나게 된다.
이것은 **C++ 동작의 정확한 재현**이지만, 사용자 관점에서는 Rust의 현재 동작이 더 좋다.

**결정**: C++ 호환성을 우선하여 rich priority를 그대로 적용한다.
향후 priority 가중치 튜닝은 별도 작업으로 분리.

---

## 5. 변경 범위 요약

| 파일 | 변경 유형 | 변경량 |
|------|----------|--------|
| `suggestion/generators.rs` | trait 시그니처 + 12 impl + SplitWord::spell_ok | ~50줄 수정 |
| `suggestion/strategy.rs` | generate() 시그니처 + 호출부 | ~10줄 수정 |
| `suggestion/vfst.rs` | generate() 시그니처 (VfstSuggestion) | ~3줄 수정 |
| `handle.rs` | suggest() 에서 analyzer 전달 | ~2줄 수정 |
| 테스트 코드 | generate() 호출부 수정 | ~15줄 수정 |

총 약 **80줄** 수정. 신규 코드 없음 (이미 구현된 인프라 활용).

---

## 6. 실행 순서 (compact 후)

1. trait 시그니처 변경 (`generators.rs`)
2. 12개 generator 구현체 수정 (`generators.rs`)
3. VfstSuggestion 시그니처 수정 (`vfst.rs`)
4. SuggestionStrategy 수정 (`strategy.rs`)
5. handle.rs 수정
6. 테스트 코드 수정
7. `suggest_for_buffer` wrapper 삭제 여부 결정
8. `cargo test --all-features` 통과 확인
9. `cargo clippy --all-features -- -D warnings` 확인
10. 차등 테스트 실행 → 결과 기록
11. commit + push

---

## 7. 참조 파일

| 파일 | 용도 |
|------|------|
| `plan/phase2-rust/06-suggest-rich-priority.md` | 이 문서 |
| `plan/phase2-rust/05-dev-guidelines.md` | 개발 철학/코딩 원칙 |
| `libvoikko/rust/crates/voikko-fi/src/suggestion/generators.rs` | Generator trait + 구현체 |
| `libvoikko/rust/crates/voikko-fi/src/suggestion/strategy.rs` | Strategy 오케스트레이션 |
| `libvoikko/rust/crates/voikko-fi/src/suggestion/vfst.rs` | VfstSuggestion |
| `libvoikko/rust/crates/voikko-fi/src/suggestion/status.rs` | SuggestionStatus |
| `libvoikko/rust/crates/voikko-fi/src/handle.rs` | VoikkoHandle.suggest() |
| `libvoikko/src/spellchecker/suggestion/SuggestionGeneratorCaseChange.cpp` | C++ 원본 (참조) |
| `libvoikko/src/spellchecker/suggestion/SuggestionStrategyTyping.cpp` | C++ strategy (참조) |
| `libvoikko/rust/tests/differential/golden/suggest.json` | C++ golden file |

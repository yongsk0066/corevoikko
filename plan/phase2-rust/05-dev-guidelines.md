# 개발 가이드라인 — Rust WASM 포팅

> 프로젝트 전반에 걸쳐 지켜야 할 철학, 코딩 원칙, 검증 방법론.

---

## Rust 버전

```
rustc 1.93.1
edition = "2024"
```

Edition 2024는 Rust 1.85부터 안정화. `unsafe_op_in_unsafe_fn` 기본 lint,
`gen` 블록, lifetime capture rules 개선 등 신규 프로젝트에 적합한 변경 포함.

---

## 1. 설계 철학

### 1.1 Screaming Architecture

코드를 열었을 때 주석 없이도 **구조 자체가 의도를 드러내야 한다**.
파일 이름, 모듈 경계, 타입 이름만으로 "이 코드가 뭘 하는지" 소리치게 만든다.

```
// Good: 파일 구조가 곧 아키텍처 문서
voikko-fi/src/
  morphology/
    finnish.rs      ← 핀란드어 형태소 분석
    tag_parser.rs   ← FST 출력 태그 파서
  speller/
    adapter.rs      ← 형태소 분석 → 맞춤법 어댑터

// Bad: 모호한 이름, 책임 불분명
voikko-fi/src/
  processor.rs
  helper.rs
  utils2.rs
```

### 1.2 High Cohesion, Low Coupling

- **모듈 하나 = 책임 하나**. `mod.rs`에서 public API만 re-export.
- 모듈 내부는 자유롭게 접근하되, 외부에는 최소한의 `pub` 인터페이스만 노출.
- 너무 파편화하지 않는다. 관련 로직이 3개 파일에 흩어지느니 한 파일에 섹션으로 나눈다.
- **판단 기준**: "이 함수를 옮기면 import가 몇 개 바뀌나?" — 많이 바뀌면 응집도가 낮은 것.

### 1.3 Make Illegal States Unrepresentable

타입 시스템으로 비즈니스 규칙을 인코딩한다. 런타임 검증보다 컴파일 타임 보장을 우선.

```rust
// Good: 상태가 타입으로 명확
enum SpellResult {
    Ok,
    CapitalizeFirst,
    CapitalizationError,
    Failed,
}

// Bad: 매직 넘버로 상태 표현
fn spell(word: &str) -> i32 { /* 0=ok, 1=cap_first, ... */ }
```

### 1.4 Parse, Don't Validate

입력을 받을 때 한 번 파싱하여 유효한 타입으로 변환하고, 이후로는 유효성을 다시 검사하지 않는다.

```rust
// Good: 파싱 결과가 곧 유효한 타입
struct VfstHeader { weighted: bool }

fn parse_header(data: &[u8]) -> Result<VfstHeader, VfstError> {
    // 여기서 한 번 검증. 이후 VfstHeader는 항상 유효.
}

// Bad: 매번 검증 반복
fn is_valid_header(data: &[u8]) -> bool { ... }
fn get_weighted(data: &[u8]) -> bool { /* is_valid 안 불렀으면? */ }
```

### 1.5 Pragmatic Functional Programming

순수 FP 도그마가 아니라, 함수형의 좋은 점을 실용적으로 취한다.

**적극 활용**:
- Iterator combinator (`.filter().map().collect()`)
- 불변 바인딩 기본 (`let` > `let mut`)
- 순수 함수 선호 (입력 → 출력, side-effect 분리)
- `Result`/`Option` 체이닝

**자제**:
- Combinator 체인 3-4단계 이상 → 중간 변수로 끊기
- 과도한 제네릭 → 구체 타입이 더 명확하면 구체 타입 사용
- `clone()` 남발 → 소유권 설계를 먼저 고민. `Cow<'_, str>` 같은 대안 검토.

```rust
// Good: 적절한 함수형 스타일
let valid_analyses: Vec<Analysis> = raw_outputs
    .iter()
    .filter_map(|output| parse_fst_output(output).ok())
    .filter(|a| is_valid_compound(a))
    .collect();

// Bad: 과도한 체이닝 — 읽기 어려움
let result = items.iter().filter(|x| x.a > 0).map(|x| {
    x.children.iter().flat_map(|c| c.values.iter().filter(|v| v.is_some()))
}).flatten().enumerate().take_while(|(i, _)| *i < 10).collect::<Vec<_>>();
```

---

## 2. Rust 코딩 원칙

### 2.1 Trait 설계

- **작게 유지**. 하나의 trait = 하나의 역할. God trait 금지.
- C++의 abstract class를 그대로 옮기지 않는다. Rust의 trait은 합성(composition)이 자연스러우므로, 필요한 행위만 분리.
- **핵심**: 중요한 trait 설계는 **내부** 추상화에 있다. 공개 API는 `Voikko` struct의 메서드로 충분하고, trait으로 나눌 필요가 없다.

```rust
// Good: 내부 추상화를 위한 trait
trait Transducer {
    type Config;
    fn prepare(&self, config: &mut Self::Config, input: &[char]) -> bool;
    fn next(&self, config: &mut Self::Config, output: &mut String) -> bool;
}

trait Analyzer {
    fn analyze(&self, word: &[char]) -> Vec<Analysis>;
}

// Bad: 공개 API를 trait으로 쪼갬 (불필요)
trait Spell { fn spell(&self, word: &str) -> SpellResult; }
trait Suggest { fn suggest(&self, word: &str) -> Vec<String>; }
// → 그냥 Voikko struct에 메서드로 두면 됨
```

### 2.2 에러 처리

- 도메인별 에러 타입 정의. `thiserror` 크레이트 활용.
- `unwrap()` / `expect()` 은 테스트와 확실히 불가능한 경우에만.
- `?` 연산자로 간결하게 전파.
- WASM 경계에서는 `Result<T, JsError>`로 변환. `thiserror` → `JsError` 변환 경로를 명확히 유지.

```rust
#[derive(Debug, thiserror::Error)]
pub enum VfstError {
    #[error("invalid magic number in VFST header")]
    InvalidMagic,
    #[error("file too short: expected at least {expected} bytes, got {actual}")]
    TooShort { expected: usize, actual: usize },
    #[error("unsupported transducer type: weighted={0}")]
    TypeMismatch(bool),
}

// WASM 경계에서의 변환
#[wasm_bindgen]
impl WasmVoikko {
    #[wasm_bindgen(constructor)]
    pub fn new(data: &[u8], lang: &str) -> Result<WasmVoikko, JsError> {
        let handle = VoikkoHandle::from_bytes(data, lang)?; // VfstError → JsError 자동 변환
        Ok(WasmVoikko { handle })
    }
}
```

### 2.3 소유권 설계

C++의 raw pointer 패턴을 기계적으로 옮기지 않는다. Rust의 소유권 모델에 맞게 재설계.

| C++ 패턴 | Rust 대체 |
|----------|----------|
| `new T` / `delete` | `Box<T>` 또는 스택 값 |
| `T*` (비소유) | `&T` 또는 `&mut T` |
| `list<Analysis*>` | `Vec<Analysis>` |
| 공유 포인터 | `Arc<T>` (필요할 때만) |
| factory 패턴 반환 포인터 | `Box<dyn Trait>` |

### 2.4 라이프타임 & 공유 소유권 전략

C++의 VoikkoHandle은 Analyzer를 소유하고, Speller/Hyphenator/Grammar가 같은 Analyzer를 빌려 쓴다.
이 공유 패턴을 Rust에서 처리하는 전략:

**기본 접근: `Arc<dyn Analyzer>`로 공유**

```rust
struct VoikkoHandle {
    analyzer: Arc<dyn Analyzer>,
    speller: Box<dyn Speller>,       // 내부에서 analyzer.clone() (Arc clone = 저렴)
    hyphenator: Box<dyn Hyphenator>, // 동일
    grammar: Box<dyn GrammarChecker>,
    options: VoikkoOptions,
}
```

- `Arc` clone은 reference count 증가일 뿐, 실제 데이터 복사 없음
- 라이프타임 지옥과 self-referential struct 문제를 피할 수 있음
- WASM은 싱글스레드이므로 `Rc`로도 충분하지만, native 빌드 호환을 위해 `Arc` 사용

**대안 검토 시기**: 벤치마크에서 `Arc` 오버헤드가 측정 가능한 수준일 때만
라이프타임 기반 설계(`&'a dyn Analyzer`)를 검토. 조기 최적화 금지.

### 2.5 문자열 전략

C++의 `wchar_t` (4바이트 고정폭)를 Rust에서 처리하는 전략:

- **외부 경계**: `&str` / `String` (UTF-8). wasm-bindgen이 JS UTF-16 ↔ Rust UTF-8 자동 변환.
- **내부 — 문자 위치 랜덤 액세스가 필요한 곳만**: `&[char]` / `Vec<char>`. STRUCTURE 파싱, 하이픈 위치 계산 등.
- **내부 — 순차 스캔이면**: `&str` + `.chars()` / `.char_indices()` 우선. 불필요한 `Vec<char>` 할당을 피한다.
- **변환 경계**: API 진입점에서 `String → Vec<char>`, 반환 시 `Vec<char> → String`.

```rust
// 문자 위치 기반 랜덤 액세스가 필요한 곳 → &[char]
fn parse_structure(fst_output: &[char], word_len: usize) -> Vec<char> { ... }
fn hyphenation_points(word: &[char]) -> Vec<bool> { ... }

// 순차 스캔이면 &str로 충분 → Vec<char> 불필요
fn is_nonword(word: &str) -> bool {
    word.chars().all(|c| c.is_ascii_digit() || c == '.')
}

// API 경계
pub fn analyze(&self, word: &str) -> Vec<Analysis> {
    let chars: Vec<char> = word.chars().collect();
    self.analyze_chars(&chars)
}
```

### 2.6 `unsafe` 정책

| 레벨 | 허용 여부 | 예시 |
|------|-----------|------|
| **검증된 크레이트 경유** | ✅ 허용 | `bytemuck::cast_slice`, `zerocopy` derive |
| **성능 크리티컬 hot path** | ⚠️ 조건부 허용 | FST 트랜지션 테이블 bounds check 제거 (벤치마크로 효과 입증 후) |
| **직접 포인터 연산** | ❌ 금지 | `*mut T` 직접 조작, `transmute` |
| **자체 unsafe 추상화** | ❌ 금지 | safe API 뒤에 unsafe를 숨기는 새 타입 작성 |

**규칙**:
- 모든 `unsafe` 블록에 `// SAFETY:` 주석 필수. 왜 안전한지 한 문장으로 설명.
- `unsafe` 사용은 PR 리뷰에서 별도 주의 항목.
- **먼저 safe하게 구현**, 벤치마크로 병목 확인 후에만 unsafe 도입 검토.

```rust
// Good: SAFETY 주석 + 검증된 크레이트
let transitions: &[Transition] = bytemuck::cast_slice(&data[offset..]);

// Good: 벤치마크로 입증된 hot path 최적화
// SAFETY: offset은 parse_header에서 검증됨, transitions 배열 범위 내
let t = unsafe { transitions.get_unchecked(index) };

// Bad: 이유 없는 unsafe
let ptr = data.as_ptr();
unsafe { *(ptr.add(offset) as *const u32) }
```

### 2.7 `#[cfg]` 분기 전략

| 조건 | 용도 |
|------|------|
| `#[cfg(target_arch = "wasm32")]` | WASM 전용 코드 (fetch, no-mmap) |
| `#[cfg(not(target_arch = "wasm32"))]` | Native 전용 코드 (memmap2) |
| `#[cfg(test)]` | 테스트 전용 |
| `#[cfg(feature = "suggest")]` | Feature flag 기반 모듈 선택 |

### 2.8 의존성 정책

**원칙: 최소 의존성.** 표준 라이브러리로 충분하면 외부 크레이트를 쓰지 않는다.
WASM 번들 크기에 직접 영향을 주므로 신규 의존성 추가는 신중하게.

**허용 목록 (확정)**:

| 크레이트 | 용도 | 크기 영향 |
|----------|------|-----------|
| `bytemuck` | zero-copy 트랜지션 파싱 | 극소 (derive 매크로만) |
| `thiserror` | 에러 타입 derive | 극소 (컴파일 타임만) |
| `hashbrown` | 심볼 테이블 HashMap | 소 (~5KB) |
| `serde` + `serde-wasm-bindgen` | 복합 타입 JS 변환 | 중 (~15-30KB) |
| `wasm-bindgen` | JS 바인딩 | voikko-wasm crate에만 |

**추가 검토 후보** (필요 시):

| 크레이트 | 용도 | 조건 |
|----------|------|------|
| `smol_str` | 심볼 테이블 small string | 벤치마크에서 String 할당이 병목일 때 |
| `unicode-normalization` | 입력 정규화 | 자체 구현 vs 크레이트 비교 후 |
| `criterion` | 벤치마크 | dev-dependency만 (번들 무관) |
| `proptest` | property-based testing | dev-dependency만 |
| `memmap2` | native mmap | `#[cfg(not(wasm32))]`로 격리 |

**신규 크레이트 추가 절차**:
1. `cargo bloat --release --crates` 로 크기 영향 측정
2. 대안 (자체 구현 or 다른 크레이트) 비교
3. 가이드라인에 추가 기록

---

## 3. 성능 가이드라인

### 3.1 핫 패스 식별

FST 순회가 이 라이브러리의 **지배적 핫 패스**다. 단어 하나당 수천~수만 번 트랜지션 탐색.

| 핫 패스 | 호출 빈도 | 성능 목표 |
|---------|-----------|-----------|
| `Transducer::next()` | 단어당 ~10,000회 | 힙 할당 0, C++과 동등 |
| 심볼 테이블 lookup | 트랜지션당 1회 | O(1) HashMap |
| Flag diacritic check | 트랜지션당 조건부 | 분기 최소화 |
| `FinnishVfstAnalyzer::parse_*` | 분석 결과당 1회 | 적절한 수준 |

### 3.2 성능 원칙

1. **먼저 정확하게, 그 다음 빠르게.** Phase별 정확성 검증 후에만 최적화.
2. **추측 기반 최적화 금지.** `criterion` 벤치마크로 병목을 측정한 후에만 최적화.
3. **FST 순회 중 힙 할당 0이 목표.** C++도 순회 중 new/delete를 하지 않는다. 출력 버퍼는 재사용.
4. **Phase 1 완료 후 벤치마크 기준선 확립.** 이후 Phase에서 성능 회귀 감지.

### 3.3 벤치마크 구성

```rust
// benches/fst_bench.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_spell_common_words(c: &mut Criterion) {
    let voikko = test_voikko();
    let words = load_wordlist("common_1000.txt");
    c.bench_function("spell_1000_words", |b| {
        b.iter(|| {
            for word in &words {
                voikko.spell(word);
            }
        })
    });
}
```

Phase 1 완료 후 벤치마크 항목:
- `fst_traverse`: 단일 단어 FST 순회 시간
- `spell_1000`: 빈출 1,000단어 맞춤법 검사
- `analyze_100`: 100단어 형태소 분석

---

## 4. 검증 방법론 — 단계별 크로스체크

**원칙**: 코드만 보고 "맞겠지" 하지 않는다. 매 Phase 완료 시 **실행 가능한 검증**을 수행.

### 4.1 Phase별 검증 체크리스트

#### Phase 1: FST 엔진

| 검증 항목 | 방법 | 합격 기준 |
|-----------|------|-----------|
| VFST 파싱 정확성 | voikko-fi 사전 `mor.vfst` 로드 성공 | 헤더, 심볼 테이블, 트랜지션 수가 C++과 동일 |
| 심볼 테이블 일치 | C++ `symbolToString[]` 덤프 vs Rust 출력 비교 | 100% 일치 |
| Flag diacritic 정확성 | 5개 연산별 단위 테스트 | P, C, U, R, D 모두 통과 |
| Unweighted 순회 | `mor.vfst`에 "koira" 입력, FST 출력 비교 | C++ `next()` 출력과 바이트 동일 |
| Weighted 순회 | 동일, weight 값 포함 비교 | 출력 + weight 모두 동일 |
| 루프 제한 | 100,000회 초과 입력 테스트 | 무한루프 없이 false 반환 |
| 벤치마크 기준선 | criterion으로 fst_traverse 측정 | 수치 기록 (회귀 감지용) |

**검증 도구**: C++ 빌드에 디버그 출력 삽입 → `(입력, FST출력)` 페어 캡처 → Rust에서 동일 입력으로 비교.

#### Phase 2: 형태소 분석 + 맞춤법

| 검증 항목 | 방법 | 합격 기준 |
|-----------|------|-----------|
| FinnishVfstAnalyzer 태그 파싱 | C++ 계측 빌드로 `(FST출력, Analysis)` 골든파일 생성 | 100개+ 단어에 대해 100% 일치 |
| STRUCTURE 문자열 | 동일 단어의 STRUCTURE 비교 | 바이트 동일 |
| 기본형 도출 | BASEFORM 비교 | 동일 |
| spell() 결과 | 10,000단어 리스트로 C++ vs Rust | 불일치 0건 |
| SpellerCache | 캐시 히트/미스 패턴 비교 (선택) | 성능만 영향, 정확성은 무관 |

**검증 도구**: Python 차등 테스트 스크립트 (`differential_test.py`).

```python
# 핵심 비교 로직
for word in wordlist:
    cpp_spell = voikko_cpp.spell(word)
    rust_spell = voikko_rust.spell(word)
    assert cpp_spell == rust_spell, f"Mismatch: {word}"
```

#### Phase 3: 추가 기능

| 기능 | 검증 방법 | 합격 기준 |
|------|-----------|-----------|
| 하이픈 | `hyphenate(word)` 비교 (1,000+ 단어) | C++과 동일 패턴 |
| 토크나이저 | `tokens(text)` 비교 (다양한 입력) | 토큰 타입 + 텍스트 동일 |
| 제안 | `suggest(word)` 비교 (100+ 오탈자) | 상위 5개 제안이 동일 순서 |

#### Phase 4: 문법 검사

| 검증 방법 | 합격 기준 |
|-----------|-----------|
| `grammarErrors(text)` 비교 (50+ 문단) | 에러 코드, 위치, 길이 동일 |
| 개별 규칙 단위 테스트 | 18개 에러 코드 각각 최소 2개 테스트 케이스 |

#### Phase 5: WASM 통합

| 검증 항목 | 방법 | 합격 기준 |
|-----------|------|-----------|
| 기존 vitest 통과 | `pnpm test` (37개 테스트) | 37/37 통과 |
| wasm-bindgen 타입 | 생성된 .d.ts 검토 | 기존 types.ts와 호환 |
| 번들 크기 | `ls -lh *.wasm` | < 500KB (gzip < 200KB) |

### 4.2 차등 테스트 인프라

프로젝트 초기(Phase 1)에 차등 테스트 인프라를 구축한다:

```
libvoikko/rust/tests/
├── differential/
│   ├── capture_cpp.py    # C++ 출력 캡처 스크립트
│   ├── compare.py        # Rust vs C++ 비교 스크립트
│   ├── wordlist.txt      # 테스트 단어 리스트 (voikko-fi에서 추출)
│   └── golden/           # C++ 출력 골든파일
│       ├── spell.json    # { "koira": true, "koirra": false, ... }
│       ├── analyze.json  # { "koira": [{ "BASEFORM": "koira", ... }], ... }
│       └── hyphenate.json
```

**단어 리스트 생성**: voikko-fi 사전의 어휘(`vocabulary/*.xml`)에서 추출하거나, 핀란드어 텍스트 코퍼스에서 빈출 단어 수집.

### 4.3 CI에서의 검증

매 PR마다:
1. `cargo test` — Rust native 테스트 전체
2. `cargo test --target wasm32-unknown-unknown` — WASM 테스트 (Phase 5부터)
3. `python compare.py` — 차등 테스트 (골든파일 기반)
4. `cargo clippy -- -D warnings` — lint
5. `cargo fmt --check` — 포맷팅

### 4.4 테스트 패턴

```rust
// 모듈 내부 #[cfg(test)]로 유닛 테스트
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_header_valid() { ... }

    #[test]
    fn parse_header_invalid_magic() { ... }
}

// tests/ 디렉토리에 통합 테스트 (사전 필요)
// 환경변수로 사전 경로 제어 — 없으면 skip
#[test]
fn spell_with_real_dictionary() {
    let dict_path = match std::env::var("VOIKKO_DICT_PATH") {
        Ok(p) => p,
        Err(_) => { eprintln!("VOIKKO_DICT_PATH not set, skipping"); return; }
    };
    // ...
}
```

---

## 5. 진행 상황 기록

### 5.1 기록 위치

- `plan/phase2-rust/00-master-plan.md` 하단 "진행 일지" 섹션에 날짜별 기록
- Phase 완료 시 검증 결과 요약 포함

### 5.2 기록 형식

```markdown
## 진행 일지

- YYYY-MM-DD: Phase X-Y 완료 — [작업 내용]
  - 검증: [검증 방법] → [결과 (통과/실패/불일치 N건)]
  - 발견: [예상치 못한 발견 사항]
```

### 5.3 Phase 완료 기준

Phase 완료 선언 전 반드시 확인:

- [ ] 모든 `cargo test` 통과
- [ ] clippy 경고 0건
- [ ] 해당 Phase의 검증 체크리스트 전항목 통과
- [ ] 차등 테스트 불일치 0건 (해당하는 경우)
- [ ] 벤치마크 기준선 기록 (해당하는 경우)
- [ ] 진행 일지에 결과 기록

---

## 6. C++ 포팅 시 주의사항

### 6.1 기계적 번역 금지

C++ 코드를 줄 단위로 Rust로 옮기지 않는다. Rust의 관용적 패턴으로 재설계한다.

| C++ 패턴 | ❌ 기계적 번역 | ✅ Rust 관용적 |
|----------|---------------|---------------|
| `for (int i = 0; i < len; i++)` | `for i in 0..len` | `.iter().enumerate()` (필요시) |
| `if (ptr == NULL)` | `if x.is_none()` | `if let Some(v) = x { ... }` |
| `switch (type)` | `if type == A { } else if ...` | `match type { A => ..., B => ... }` |
| `goto nextInMainLoop` | `unsafe { goto!() }` | `continue 'outer` (labeled loop) |
| `new T() / delete` | `Box::new(T::new())` / 수동 drop | 스택 값 또는 `Vec`, Drop 자동 |

### 6.2 C++ 코드 참조 방법

모든 Rust 함수에 원본 C++ 위치를 주석으로 남기되, **구현이 아닌 위치와 함수명만** 표기:

```rust
/// Parses VFST binary header.
/// Origin: Transducer::vfstMmap() — Transducer.cpp:163-178
fn parse_header(data: &[u8]) -> Result<VfstHeader, VfstError> {
    // Rust 관용적 구현 (C++ 코드의 직역이 아님)
}
```

### 6.3 핀란드어 도메인 지식

핀란드어 언어학 상수(모음 테이블, 격변화 맵 등)는 별도 `const` 모듈에 집중 배치:

```rust
// voikko-fi/src/finnish/constants.rs
pub const BACK_VOWELS: &[char] = &['a', 'o', 'u'];
pub const FRONT_VOWELS: &[char] = &['ä', 'ö', 'y'];
pub const SPLIT_VOWEL_PAIRS: &[(&str, &str)] = &[("ae", ""), ("ao", ""), ...];
```

이렇게 하면:
- 상수 변경 시 한 곳만 수정
- 핀란드어 지식이 없어도 로직과 데이터를 분리해서 읽을 수 있음
- 테스트에서 상수를 직접 참조 가능

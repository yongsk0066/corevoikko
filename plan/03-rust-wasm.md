# Rust + wasm-bindgen 분석

**종합 점수: 7.5/10** -- 장기 모던화에 최적, 투자 대비 효과 높음

## 포팅 범위

| 모듈 | 라인 수 | Rust 포팅 난이도 | 비고 |
|------|-------:|:---------------:|------|
| fst/ | 1,556 | 중 | 바이너리 파싱 + 비트필드. Rust에 자연스럽게 매핑 |
| morphology/ | 2,525 | 중-상 | FinnishVfstAnalyzer 1,179줄 복잡 |
| spellchecker/ | 2,368 | 중 | VfstSpeller 단순. suggestion 전략 다수 |
| grammar/ | 3,395 | 상 | 핀란드어 규칙 엔진 + 6개 체크 모듈 |
| hyphenator/ | 1,197 | 중 | 음절 규칙 테이블 |
| tokenizer/ | 376 | 하 | 단순 |
| setup/ | 2,119 | 중 | 사전 로딩, 팩토리 |
| character/ | 635 | 하 | Rust char/Unicode 우수 |
| utils/ | 602 | 하 | Rust std로 대체 가능 |

## VFST 바이너리 파싱 -> Rust

### 구조체 매핑

```rust
// Unweighted Transition (8 bytes)
// C++ 비트필드 transinfo_t { targetState:24, moreTransitions:8 }
// -> Rust에서 비트 마스킹으로 수동 처리
fn target_state(raw: u32) -> u32 { raw & 0x00FF_FFFF }
fn more_transitions(raw: u32) -> u8 { (raw >> 24) as u8 }

// Weighted Transition (16 bytes) -> 직접 매핑 가능
#[repr(C)]
struct WeightedTransition {
    sym_in: u32,
    sym_out: u32,
    target_state: u32,
    weight: i16,
    more_transitions: u8,
    reserved: u8,
}
```

### 엔디언 처리
- `byteorder` 크레이트로 LE/BE 투명 처리
- WASM은 항상 little-endian -> LE VFST면 바이트 스와핑 불필요

### mmap 대체
- WASM에서는 `&[u8]` 슬라이스로 직접 처리 (전체 파일 메모리 로드)
- 네이티브 빌드에서는 `memmap2` 크레이트 사용 가능

## wasm-bindgen 통합

```rust
#[wasm_bindgen]
pub struct Voikko { /* ... */ }

#[wasm_bindgen]
impl Voikko {
    #[wasm_bindgen(constructor)]
    pub fn new(lang: &str, dict_data: &[u8]) -> Result<Voikko, JsValue> { ... }

    pub fn spell(&self, word: &str) -> bool { ... }
    pub fn suggest(&self, word: &str) -> Vec<String> { ... }
    pub fn hyphenate(&self, word: &str) -> String { ... }
    pub fn analyze(&self, word: &str) -> JsValue { ... }
}
```

- Rust UTF-8 -> JS UTF-16 변환: wasm-bindgen 자동 처리
- Vec<String> -> JS Array 자동 변환
- 복잡한 객체: serde-wasm-bindgen으로 직렬화

## 메모리 관리 개선

| C++ 패턴 | 빈도 | Rust 대체 |
|----------|------|----------|
| `new wchar_t[N]` / `delete[]` | 많음 | `Vec<char>` / `String` |
| `new Configuration(...)` / `delete` | 모듈별 | 구조체 소유권 (자동 Drop) |
| `new Transducer(...)` / `terminate()` | 모듈별 | `impl Drop` |
| `list<Analysis*>` / `deleteAnalyses()` | 분석별 | `Vec<Analysis>` |
| mmap / munmap | 파일 로딩 | WASM: `Vec<u8>`, native: `memmap2` |

핵심 이점: 447개 수동 new/delete -> 소유권 + Drop으로 자동화, 메모리 누수 원천 차단

## 번들 사이즈

| 항목 | 크기 |
|------|------|
| WASM 바이너리 (wasm-opt -Oz) | 200-400 KB |
| gzip 압축 | 80-150 KB |
| 사전 데이터 | 별도 (~3.9 MB) |

최적화 설정:
```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
```

모듈별 선택적 빌드: Cargo feature flags로 spell-only, analyze-only 등 가능

## 성능

| 기준 | Rust WASM | C++ Emscripten |
|------|-----------|----------------|
| 실행 속도 | ~1.0x native | ~1.0-1.2x native |
| 문자열 전달 | 빠름 (wasm-bindgen) | 중간 (수동 변환) |
| 메모리 | 낮음 (zero-cost) | 중간 (런타임 오버헤드) |
| JS 호출 오버헤드 | 낮음 | 중간 |

## 기존 바인딩 호환

```rust
// C FFI 레이어로 기존 Python/Java 바인딩 유지 가능
#[no_mangle]
pub extern "C" fn voikkoInit(
    error: *mut *const c_char,
    langcode: *const c_char,
    path: *const c_char
) -> *mut VoikkoHandle { ... }
```

- `cbindgen`으로 voikko.h 자동 생성
- Python ctypes, Java JNA 동일 방식 로드
- 동일 Rust 코드베이스에서 네이티브 + WASM + C FFI 모두 빌드

## Rust FST 생태계

| 크레이트 | corevoikko 적합성 |
|---------|-------------------|
| fst (BurntSushi) | 부적합 - VFST와 다른 포맷 |
| rustfst | 참고용 - 가중 트랜스듀서 구현 참고 |
| **divvunspell** | **핵심 레퍼런스** - 핀우그르어 FST 스펠체커 Rust WASM 성공 선례 |
| byteorder | VFST 바이트 스와핑에 직접 사용 |

## 점진적 포팅 전략 (권장)

```
1단계: fst/ 엔진              (2-3주) -- 모든 모듈의 기반
2단계: character/ + utils/    (1주)   -- 유틸리티 계층
3단계: spellchecker/          (2-3주) -- 가장 유용한 기능 -> MVP
4단계: morphology/            (3-4주) -- 분석 기능
5단계: hyphenator/            (1-2주)
6단계: tokenizer/             (1주)
7단계: grammar/               (3-4주) -- 가장 복잡
```

각 단계에서 독립적 WASM 빌드 + 테스트 가능.
MVP (1-3단계): 4-6주로 spell check WASM 패키지 출시 가능.

## 작업량 추정

| 단계 | 예상 기간 |
|------|----------|
| FST 엔진 | 2-3주 |
| 유틸리티/캐릭터 | 1주 |
| Spellchecker | 2-3주 |
| Morphology | 3-4주 |
| Hyphenator | 1-2주 |
| Grammar | 3-4주 |
| Tokenizer/Setup | 1주 |
| wasm-bindgen API + JS 래퍼 | 1-2주 |
| 테스트 + 검증 | 2-3주 |
| **합계** | **16-23주 (1명 풀타임)** |

## 장점

1. wasm-bindgen: 최상의 JS DX (TS 타입 자동생성)
2. 소유권 모델: 447개 수동 메모리 관리 근본 해결
3. 단일 코드베이스: 네이티브 + WASM + C FFI
4. divvunspell: 동일 도메인 성공 선례
5. Cargo feature flags: 모듈별 선택 빌드
6. UTF-8 네이티브: wchar_t 의존성 제거

## 단점

1. 16-23주 대규모 재작성 (Emscripten 대비 10배+)
2. FinnishVfstAnalyzer 파싱 로직 이식 시 호환성 버그 리스크
3. 핀란드어 문법 도메인 지식 필요
4. Rust 학습 곡선 (기존 C++ 기여자)
5. upstream 동기화 불가 (포크 유지보수)

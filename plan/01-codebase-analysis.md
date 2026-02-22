# 코드베이스 현황 분석

## 프로젝트 개요

Corevoikko: 핀란드어 NLP 오픈소스 라이브러리 (맞춤법 검사, 하이픈 처리, 문법 검사, 형태소 분석)

- **libvoikko**: C++ 핵심 라이브러리
- **voikko-fi**: VFST 형식 핀란드어 형태소 사전
- **라이선스**: MPL 1.1 / GPL 2+ / LGPL 2.1+ (tri-license)

## 코드 규모

| 언어 | 파일 수 | 라인 수 |
|------|------:|-------:|
| C++ (.cpp/.hpp/.h) | 210 | ~22,580 |
| Python | 27 | ~6,650 |
| Java | 17 | ~2,329 |
| **합계** | **254** | **~31,559** |

### 모듈별 코드 규모

| 모듈 | 라인 수 | 역할 |
|------|-------:|------|
| fst/ | 1,556 | FST 엔진 (VFST/HFST 트랜스듀서) |
| morphology/ | 2,525 | 형태소 분석 |
| spellchecker/ + suggestion/ | 4,983 | 맞춤법 검사 + 제안 생성 (12개 전략) |
| grammar/ + FinnishRuleEngine/ | 4,581 | 문법 검사 (15+ 규칙) |
| hyphenator/ | 1,197 | 하이픈 처리 |
| tokenizer/ | 376 | 토크나이저 |
| character/ | 635 | 유니코드 케이스 변환 |
| setup/ | 2,119 | 사전 로딩, 초기화 |
| utils/ | 602 | 문자열 유틸리티 |

## C++ 표준 현황

### 설정: C++17 필수

```
# configure.ac:54
AX_CXX_COMPILE_STDCXX(17, noext, mandatory)
```

### 실제 사용 현황: C++17 기능을 거의 사용하지 않음

| 기능 | 사용 여부 | 횟수 |
|------|----------|------|
| std::optional | 미사용 | 0 |
| std::string_view | 미사용 | 0 |
| std::filesystem | 미사용 | 0 |
| if constexpr | 미사용 | 0 |
| structured bindings | 미사용 | 0 |
| std::variant | 미사용 | 0 |
| smart pointers (unique/shared) | **미사용** | **0** |
| range-based for | 사용 | 6 |
| auto | 최소 | 2개 파일 |
| nullptr | 일부 | 15 |
| constexpr | 일부 | 15 |
| **raw new/delete** | **지배적** | **447** |
| NULL/= 0 패턴 | 광범위 | 363 |

**결론**: C++17 요구하지만 실제 코드 스타일은 "C-with-classes" (C++03 수준)

## 메모리 관리

- **수동 new/delete**: 447회 (92개 파일) - 지배적 패턴
- **smart pointer**: 0회 - 전혀 미사용
- **mmap**: 사전 로딩에 사용 (Unix: mmap, Windows: CreateFileMapping)
- **C 메모리 함수**: malloc/calloc 10회, printf/fprintf 19회

### 주요 메모리 관리 패턴

```cpp
// 팩토리: 호출자가 delete 책임
static Analyzer * getAnalyzer(const setup::Dictionary & dictionary);

// 수동 해제
void Analyzer::deleteAnalyses(list<Analysis *> * &analyses) {
    list<Analysis *>::iterator it = analyses->begin();
    while (it != analyses->end()) {
        delete *it++;
    }
    delete analyses;
    analyses = 0;
}

// Configuration: 생성자에서 7개 new[], 소멸자에서 7개 delete[]
```

## 스레딩 모델

- **단일 스레드**: 핸들(VoikkoHandle) 단위
- voikko.h: "A single handle should not be used simultaneously from multiple threads."
- Java 바인딩만 `synchronized`로 스레드 안전 래핑

## 에러 처리

- **혼합**: C++ 예외 (`DictionaryException`) + C API 에러코드
- 초기화: 예외 -> 출력 매개변수 래핑
- 런타임: enum 반환값 (SPELL_FAILED, SPELL_OK 등)

## 공개 API

- **voikko.h**: C 함수 45개
  - 초기화/종료: 2개
  - 맞춤법 검사: 4개
  - 하이픈: 3개
  - 문법 검사: 7개
  - 형태소 분석: 6개
  - 토크나이저: 4개
  - 사전 관리: 7개
  - 설정: 2개
  - 메모리 해제: 4개
  - 유틸: 1개

## 바인딩

| 언어 | 기술 | 스레드 안전 |
|------|------|------------|
| Python | ctypes (CDLL) | 단일 스레드 |
| Java | JNA | synchronized |
| JS (Emscripten) | cwrap | 단일 스레드 |

## 외부 의존성

| 의존성 | 필수 여부 | 용도 |
|--------|----------|------|
| C++17 컴파일러 | 필수 | 빌드 |
| POSIX (mmap, stat) | 필수 (Unix) | 파일 처리 |
| hfstospell >= 0.5 | 선택 | HFST 백엔드 |
| lttoolbox >= 3.2 | 선택/실험 | Lttoolbox 백엔드 |
| VisCG3 + tinyxml2 | 선택/실험 | CG3 문법 백엔드 |
| pthread | 조건부 | 스레드 지원 |

## VFST 바이너리 포맷

```
[Header: 16 bytes]
  - Magic cookie 1: uint32 (0x00013A6E LE / 0x6E3A0100 BE)
  - Magic cookie 2: uint32 (0x000351FA LE / 0xFA510300 BE)
  - Weighted flag: byte 8
  - Reserved: 4 bytes

[Symbol Table]
  - uint16: symbol count
  - N개 null-terminated UTF-8 strings
  - '@' prefix: flag diacritics
  - '[' prefix: multi-character symbols

[Padding to 8-byte boundary]

[Transition Table]
  Unweighted (8 bytes each):
    - symIn: uint16
    - symOut: uint16
    - targetState: 24-bit bitfield
    - moreTransitions: 8-bit

  Weighted (16 bytes each):
    - symIn: uint32
    - symOut: uint32
    - targetState: uint32
    - weight: int16
    - moreTransitions: uint8
    - reserved: uint8
```

## C++23 업그레이드 가능성

### Phase 1: C++17 실제 활용
- std::optional -> null 반환 패턴 대체
- std::string_view -> 문자열 복사 감소
- structured bindings -> map 순회 정리

### Phase 2: 메모리 관리 현대화
- 447개 raw new/delete -> unique_ptr/shared_ptr
- list<Analysis*> -> vector<unique_ptr<Analysis>>
- 팩토리 반환 -> unique_ptr

### Phase 3: C++23 전환
- std::expected<T,E> -> 에러 처리 통합
- std::flat_map -> 심볼 테이블 성능 개선
- std::print -> printf 대체
- std::ranges -> 파이프라인 가독성

### 리스크
- GCC 13+ / Clang 17+ 필요
- -Wall -Werror -pedantic으로 새 경고가 빌드를 깨뜨릴 수 있음

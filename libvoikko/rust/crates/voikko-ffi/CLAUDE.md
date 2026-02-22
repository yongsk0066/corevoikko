# voikko-ffi

C-compatible FFI 레이어. Python/C#/Common Lisp 등 외부 언어 바인딩의 공통 기반.

## 역할

- VoikkoHandle을 opaque C 포인터로 노출
- NULL-terminated 문자열/배열 반환 패턴
- 전용 free 함수로 메모리 관리
- 30+ extern "C" 함수 (spell, suggest, analyze, hyphenate, grammar, tokens, sentences, 14 setters)

## C 헤더

`include/voikko.h` — 모든 공개 API 선언

## 빌드

```bash
# 공유 라이브러리 빌드
cargo build --release -p voikko-ffi

# 결과물
# macOS: target/release/libvoikko_ffi.dylib
# Linux: target/release/libvoikko_ffi.so
# Windows: target/release/voikko_ffi.dll
```

## 메모리 규칙

| 반환 타입 | 해제 함수 |
|-----------|-----------|
| `*mut c_char` | `voikko_free_str()` |
| `*mut *mut c_char` | `voikko_free_str_array()` |
| `VoikkoAnalysisArray` | `voikko_free_analyses()` |
| `VoikkoGrammarErrorArray` | `voikko_free_grammar_errors()` |
| `VoikkoTokenArray` | `voikko_free_tokens()` |
| `VoikkoSentenceArray` | `voikko_free_sentences()` |
| `voikko_version()` | 해제 금지 (static) |
| `voikko_attribute_values()` | 해제 금지 (static) |

## crate-type

`cdylib` + `staticlib` — 동적/정적 링킹 모두 지원

# voikko-core

공유 타입과 유틸리티. 모든 다른 crate이 의존하는 기반 모듈.

## 모듈 구조

| 모듈 | C++ 원본 | 역할 |
|------|----------|------|
| `enums` | `voikko_enums.h`, `voikko_defines.h` | TokenType, SentenceType, SpellResult, 옵션 상수 |
| `analysis` | `morphology/Analysis.hpp` | 형태소 분석 결과 (`HashMap<String, String>`) |
| `character` | `character/SimpleChar.hpp`, `charset.hpp` | 문자 분류, 핀란드어 문자 처리 |
| `case` | `utils/utils.hpp` | 대소문자 타입 감지, 변환 |
| `grammar_error` | `grammar/VoikkoGrammarError.hpp` | 문법 오류 공개 API 타입 |
| `token` | `grammar/Token.hpp`, `Sentence.hpp` | 토큰/문장 공개 API 타입 |

## 설계 원칙

- 외부 의존성: `thiserror`만 사용
- 모든 타입은 `#[derive(Debug, Clone, PartialEq)]` 기본 적용
- 공개 API 타입은 여기서 정의, 다른 crate에서 re-export
- `wchar_t` → `char` 매핑. 문자열은 `&str`/`String` 기본, 랜덤 액세스 필요 시 `&[char]`

## 빌드 & 테스트

```bash
cargo test -p voikko-core
cargo clippy -p voikko-core -- -D warnings
```

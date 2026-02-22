# voikko-fi

핀란드어 특화 모듈. 형태소 분석, 맞춤법 검사, 하이픈, 제안, 문법 검사.

## 계획된 모듈 구조 (Phase 2~4)

```
src/
  morphology/        # Phase 2-A
    mod.rs           # Analyzer trait
    vfst.rs          # VfstAnalyzer (generic weighted FST)
    finnish.rs       # FinnishVfstAnalyzer (1,179줄, 최고 난이도)
    tag_parser.rs    # FST 출력 태그 파서 (독립 단위 테스트)
  speller/           # Phase 2-B
    mod.rs           # Speller trait
    adapter.rs       # AnalyzerToSpellerAdapter
    cache.rs         # SpellerCache
    finnish.rs       # FinnishSpellerTweaks
    pipeline.rs      # 정규화 → 캐시 → spell 파이프라인
    utils.rs         # STRUCTURE 매칭
  hyphenator/        # Phase 3-A
  tokenizer/         # Phase 3-B
  suggestion/        # Phase 3-C
  grammar/           # Phase 4
  finnish/
    constants.rs     # 핀란드어 상수 (모음, 자음 테이블)
```

## Feature Flags

- `spell` (default) — 맞춤법 검사
- `analyze` (default) — 형태소 분석
- `suggest` — 제안 생성
- `hyphenate` — 하이픈 처리
- `grammar` — 문법 검사
- `tokenize` — 토크나이저

## 핵심 리스크

FinnishVfstAnalyzer의 FST 출력 태그 파싱이 최대 리스크.
골든파일 테스트와 C++ 계측 빌드로 정확성을 검증해야 함.

## 빌드 & 테스트

```bash
cargo test -p voikko-fi
cargo clippy -p voikko-fi -- -D warnings
# 통합 테스트 (사전 필요)
VOIKKO_DICT_PATH=/path/to/dict cargo test -p voikko-fi
```

상세 분석: `plan/phase2-rust/03-modules-analysis.md`

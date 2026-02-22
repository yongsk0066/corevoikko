# voikko-fst

언어 무관한 VFST (Voikko Finite State Transducer) 엔진.

## 모듈 구조

| 모듈 | C++ 원본 | 역할 | 난이도 |
|------|----------|------|--------|
| `format` | `Transducer.cpp:163-178` | 16B 헤더, 바이너리 파싱 | 낮음 |
| `transition` | `Transition.hpp`, `WeightedTransition.hpp` | `#[repr(C)]` + bytemuck zero-copy | 낮음 |
| `symbols` | `UnweightedTransducer.cpp:125-189` | 심볼 테이블 (HashMap<char, u16>) | 낮음 |
| `flags` | `Transducer.cpp:62-123` | Flag diacritics (P,C,U,R,D) | 낮음 |
| `config` | `Configuration.hpp/cpp` | 순회 상태 스택 | 낮음 |
| `unweighted` | `UnweightedTransducer.cpp:228-370` | Unweighted FST 순회 | 중간 |
| `weighted` | `WeightedTransducer.cpp:230-428` | Weighted FST 순회 (backtrack) | 중간 |

## 핵심 설계 결정

- **Byte-swap 제거**: WASM은 항상 LE, 사전도 LE로 작성됨
- **mmap 제거**: WASM은 `Vec<u8>`, native는 나중에 `memmap2` 추가
- **Trait 기반**: `Transducer` trait로 unweighted/weighted 통합
- **Zero-copy**: 트랜지션 테이블은 `bytemuck::cast_slice`로 직접 매핑
- **C++ goto 패턴 → labeled loop**: `continue 'outer` 사용

## VFST 바이너리 포맷 요약

- 16B 헤더: magic(8B) + weighted_flag(1B) + reserved(7B)
- 심볼 테이블: count(2B) + null-terminated UTF-8 strings
- 패딩: unweighted=8B, weighted=16B 경계
- 트랜지션: unweighted=8B(symIn,symOut,transInfo), weighted=16B(+weight)

## 성능 핫 패스

`Transducer::next()` — 단어당 ~10,000회 호출. 순회 중 힙 할당 0이 목표.

## 빌드 & 테스트

```bash
cargo test -p voikko-fst
cargo clippy -p voikko-fst -- -D warnings
```

상세 스펙: `plan/phase2-rust/02-fst-engine.md`

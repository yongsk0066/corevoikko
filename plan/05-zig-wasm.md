# Zig + WASM 분석

**종합 점수: 5.5/10** -- 기술적으로 흥미로우나 시기상조

## Zig WASM 지원 현황

- WebAssembly: Zig **Tier 1 타겟** (`wasm32-freestanding`, `wasm32-wasi`)
- LLVM 백엔드 사용 -> 최적화된 WASM 바이너리
- **리스크**: Zig pre-1.0 (최신 0.15.2, 2025.10). 1.0은 2026 후반 예상
- LLVM에서 자체 백엔드로 분리 계획 -> WASM 타겟 일시적 불안정 가능

## 코드 매핑 평가

corevoikko의 "C-with-classes" 스타일은 Zig에 가장 자연스럽게 매핑:

| C++ 패턴 | Zig 대응 |
|----------|---------|
| 가상 함수 (극소) | comptime 디스패치 또는 함수 포인터 |
| 템플릿 없음 | Zig comptime generics |
| 예외 (제한적) | error union |
| std::map/vector/list | HashMap, ArrayList |
| new[]/delete[] (447개) | allocator 패턴 + defer |
| wchar_t | u32 또는 u21 |
| reinterpret_cast | @ptrCast |
| memcpy/memset | @memcpy, @memset |

## VFST 파싱 -- Zig의 강점

```zig
const Transition = packed struct {
    sym_in: u16,
    sym_out: u16,
    target_state: u24,
    more_transitions: u8,
};

const WeightedTransition = packed struct {
    sym_in: u32,
    sym_out: u32,
    target_state: u32,
    weight: i16,
    more_transitions: u8,
    reserved: u8,
};
```

`packed struct`가 비트필드를 정확히 표현. C++ 비트필드보다 명시적이고 안전.
바이트 오더: `@byteSwap` 빌트인으로 간단 대체.

## 메모리 관리

- `std.heap.wasm_allocator`: WASM 전용 페이지 알로케이터
- `std.heap.ArenaAllocator`: 트랜잭션 단위 할당/해제 -> corevoikko 패턴에 이상적
- `defer` 패턴: Configuration의 7개 배열 해제를 정확히 보장

## 번들 사이즈

동일 프로젝트 기준 Zig WASM이 Emscripten 대비 ~25% 작음:
- Emscripten: ~500KB-1MB (libc + POSIX 에뮬레이션 포함)
- Zig: ~200-400KB (필요한 것만 포함)
- `ReleaseSmall` 모드로 추가 최적화 가능

## 성능

- LLVM 공유 -> Emscripten과 동등한 성능
- Zig 고유 이점:
  - `comptime`: 심볼 테이블 컴파일 타임 생성
  - 선택적 경계 검사 비활성화 (`@setRuntimeSafety(false)`)
  - 예외 핸들링 오버헤드 없음

## JS 바인딩 -- 핵심 약점

**wasm-bindgen 같은 도구 없음.** 수동 글루 코드 필요:

```
JS -> Zig WASM:
  1. TextEncoder로 UTF-8 인코딩
  2. Zig alloc 함수로 WASM 메모리 할당
  3. Uint8Array로 복사
  4. 포인터 + 길이 전달

Zig WASM -> JS:
  1. Zig가 결과를 WASM 메모리에 기록
  2. JS가 Uint8Array 뷰로 읽기
  3. TextDecoder로 UTF-8 디코딩
```

15개 핵심 함수 * 글루 코드 = ~400-600줄 수동 작성

## 생태계 성숙도 -- 핵심 리스크

| 항목 | 상태 | 영향 |
|------|------|------|
| Zig 버전 | pre-1.0 | 매 버전 breaking changes |
| 패키지 매니저 | 실험적 | 의존성 관리 불편 |
| IDE 지원 | ZLS (제한적) | 개발 속도 저하 |
| WASM 디버깅 | 제한적 | 디버깅 어려움 |
| 커뮤니티 | Rust 대비 ~1/20 | 참고 자료 부족 |
| WASM 프로덕션 사례 | 소규모 | 검증 부족 |

## 작업량 추정

| 단계 | 기간 |
|------|------|
| Zig 학습 + WASM 기초 | 3-4주 |
| FST 엔진 포팅 | 3-4주 |
| 형태소 분석기 | 2-3주 |
| 맞춤법 검사기 | 2-3주 |
| 제안 시스템 | 3-4주 |
| 문법 검사기 | 4-5주 |
| 토크나이저/하이프네이터 | 2주 |
| JS 글루 코드 + API | 2-3주 |
| 테스트 + 디버깅 | 3-4주 |
| **합계** | **24-34주 (6-8개월)** |

## 장점

1. C 스타일 코드에 구문적으로 가장 자연스러운 매핑
2. packed struct가 VFST 비트필드에 이상적
3. Allocator 시스템이 수동 new/delete 안전하게 대체
4. 번들 사이즈 Emscripten보다 ~25% 작을 가능성
5. LLVM 공유로 동등한 성능
6. 런타임 오버헤드 최소

## 단점

1. **pre-1.0 언어: 프로덕션 리스크 높음**
2. Breaking changes로 유지보수 비용 증가
3. wasm-bindgen 없음: JS 바인딩 수동 작성
4. 팀 전원 Zig 학습 필요 (3-4주)
5. LLVM 분리 시 WASM 타겟 불안정 가능
6. 참고할 대규모 Zig WASM 프로젝트 부족
7. 디버깅 도구 미성숙

## 결론

기술적으로 가능하고 코드 매핑이 가장 자연스럽지만, **언어 불안정성과 생태계 미성숙** 때문에 프로덕션에는 시기상조.
Zig 1.0 (2026 후반 예상) 이후 재평가 권장.

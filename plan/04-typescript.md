# Pure TypeScript 재구현 분석

**종합 점수: 5.5/10** -- 최고의 DX이지만 성능/정확성/유지보수 리스크 높음

## 포팅 범위

전체 22,580 라인 C++ -> TypeScript 완전 재작성. 코드 재사용 불가.

| 모듈 | 라인 수 | 난이도 |
|------|-------:|:------:|
| fst/ | 1,556 | 높음 -- 바이너리 파싱 + 포인터 연산 |
| morphology/ | 2,525 | 중-높음 -- FinnishVfstAnalyzer 1,179줄 복잡 |
| spellchecker/ + suggestion/ | 4,983 | 중 -- 다수 전략 클래스, 로직 직관적 |
| grammar/ + FinnishRuleEngine/ | 4,581 | 중 -- 규칙 기반, 문자열 패턴 매칭 |
| hyphenator/ | 1,197 | 중 -- 음절 규칙, 형태소 분석 의존 |
| tokenizer/ | 376 | 낮음 -- 단순 문자 분류 |
| character/ | 635 | 낮음 -- JS 빌트인으로 대체 가능 |
| setup/ | 2,119 | 중 -- 사전 로딩, 초기화 |
| utils/ | 602 | 낮음 |

## VFST 바이너리 파싱

ArrayBuffer + DataView로 구현:

```typescript
// 16바이트 헤더
const magic1 = dataView.getUint32(0, true);
const magic2 = dataView.getUint32(4, true);

// Transition 구조체 (8 bytes) - 24-bit 비트필드 수동 추출
const raw = dataView.getUint32(offset + 4, true);
const targetState = raw & 0x00FFFFFF;
const moreTransitions = (raw >>> 24) & 0xFF;
```

DataView 장점: 엔디언을 매개변수로 지정 가능 -> 바이트 스와핑 패스 불필요

## FST 엔진 매핑

| C++ 패턴 | TypeScript 대체 |
|----------|----------------|
| `Transition*` 포인터 연산 | `baseOffset + index * TRANSITION_SIZE` |
| 구조체 필드 접근 | `dataView.getUint16(offset)` |
| 스택 배열 (stateIndexStack 등) | `Uint32Array` / `Uint16Array` |
| `wchar_t` 출력 | JS string (점진적 구축) |
| `std::map<wchar_t, uint16_t>` | `Map<number, number>` -- O(1) hash vs O(log n) tree |

## 문자열 처리

- JS: UTF-16 내부 인코딩
- 핀란드어 문자: 모두 BMP -> UTF-16 == UCS-2 (surrogate pair 없음)
- VFST 심볼 테이블: `TextDecoder('utf-8')`로 디코딩
- 문자 접근: `string.charCodeAt(i)`가 `wchar_t` 대체

## 성능 예상

| 연산 | C++ (native) | TypeScript (V8) | 비율 |
|------|-------------|-----------------|------|
| FST 트래버설 (단어당) | ~1-5 us | ~10-50 us | 5-15x 느림 |
| 심볼 룩업 | ~0.5 us | ~0.3 us | 오히려 빠름 (hash) |
| 제안 생성 | ~1-5 ms | ~10-40 ms | 5-10x 느림 |
| 사전 로딩 | ~5 ms (mmap) | ~50-200 ms | 10-40x 느림 |

인터랙티브 사용에는 충분 (단어당 <1ms), 배치 처리에는 부적합.

## 번들 사이즈

| 항목 | 크기 (minified) | Gzip |
|------|---------------:|-----:|
| FST 엔진 | ~5 KB | ~2 KB |
| Spellchecker | ~8 KB | ~3 KB |
| Morphology | ~15 KB | ~6 KB |
| Grammar | ~20 KB | ~8 KB |
| Hyphenation | ~8 KB | ~3 KB |
| **코드 합계** | **~50 KB** | **~20 KB** |
| 사전 데이터 | ~3.9 MB | ~1.5 MB |

**Tree-shaking 장점**: 필요한 기능만 import 가능
```typescript
import { createSpellChecker } from 'voikko-ts'; // ~15 KB + 4 MB dict
```

## 개발 경험 (DX)

- npm install 한 방으로 설치
- Node.js, 브라우저, Deno, Bun, Cloudflare Workers 어디서든 동작
- TypeScript 타입으로 자동완성/문서화
- vitest/jest로 테스트
- source map 디버깅
- HMR 지원

## 유사 프로젝트

| 프로젝트 | 접근 | 성능 |
|---------|------|------|
| nspell | Pure JS, Hunspell 호환 | native 대비 ~10x 느림 |
| typo-js | Pure JS, Hunspell affix | 제안 매우 느림 (7s+) |
| hunspell-asm | WASM | 거의 native |
| cspell | Pure TS, trie 기반 | IDE 사용 충분 |

nspell/cspell이 순수 JS 맞춤법 검사 가능성을 입증. 단, 핀란드어 FST 기반 분석은 별도.

## 한계 및 리스크

1. **사전 크기**: mor.vfst 3.8 MB -> 브라우저에서 lazy loading 필수
2. **초기화 시간**: mmap 없음 -> Node.js 50-200ms, 브라우저 200-1000ms
3. **메모리**: 사전 전체를 힙에 보유 (~8-12 MB)
4. **GC 압력**: 형태소 분석 시 중간 문자열 객체 대량 생성
5. **정확성**: FinnishVfstAnalyzer 1,179줄의 정밀 파싱 -> 미묘한 버그 높은 리스크
6. **HFST 미지원**: 순수 TS로는 hfstospell 사용 불가
7. **유지보수**: upstream 변경마다 수동 포팅 -> 영구적 이중 유지보수

## 작업량 추정

| 컴포넌트 | 기간 (person-weeks) |
|---------|-------------------:|
| VFST 바이너리 파서 | 1-2 |
| UnweightedTransducer | 1-2 |
| WeightedTransducer | 1-2 |
| Configuration / 상태 관리 | 0.5 |
| Spellchecker | 0.5 |
| Suggestion (VfstSuggestion + 에러 모델) | 1 |
| Suggestion 전략 (10개 클래스) | 2 |
| FinnishVfstAnalyzer | 3-4 |
| Grammar checker | 3-4 |
| Hyphenator | 1-2 |
| Tokenizer | 0.5 |
| Character utilities | 0.5 |
| Dictionary loading / setup | 1 |
| Public API + TypeScript types | 1 |
| 테스트 + 검증 | 4-6 |
| **합계** | **21-32주 (5-8개월)** |

## 장점

1. 최고의 DX: npm, tree-shaking, TypeScript, 유니버설 런타임
2. 네이티브 의존성 0
3. 코드 크기 매우 작음 (~50 KB minified)
4. 모듈별 선택적 import 가능

## 단점

1. 5-15x 성능 저하
2. 5-8개월 재작성
3. FinnishVfstAnalyzer 정확성 리스크 높음
4. upstream 영구 분기 -> 이중 유지보수
5. 사전 크기가 지배적이라 코드 크기 이점 제한적

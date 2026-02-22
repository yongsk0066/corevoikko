# Emscripten (C++ -> WASM) 분석

**종합 점수: 8.4/10** -- 가장 현실적이고 즉시 실행 가능한 접근법

## 핵심 발견: 이미 공식 Emscripten 빌드 존재

`libvoikko/js/` 디렉토리에 완전한 빌드 파이프라인이 있음:

| 파일 | 역할 |
|------|------|
| `js/configure.sh` | emconfigure 래퍼 |
| `js/build.sh` | 3가지 빌드 모드 (embed/preload/plain) |
| `js/libvoikko_api.js` | JS 바인딩 (388줄, cwrap 기반) |
| `js/commonjs-footer.js` | Node.js CommonJS 모듈 export |
| `js/qunit.html` | QUnit 테스트 25+ 개 |

C++ 소스에 `__EMSCRIPTEN__` 가드도 이미 존재 (`DictionaryFactory.cpp:191`).

## 빌드 프로세스

```bash
source /path/to/emsdk/emsdk_env.sh
cd libvoikko
./autogen.sh
js/configure.sh   # emconfigure ./configure --with-dictionary-path=/ \
                   #   --disable-hfst --disable-buildtools --disable-testtools \
                   #   --disable-assert --disable-shared --enable-static
js/build.sh preload  # embed | preload | plain
```

- autotools가 emconfigure/emmake와 호환 -> CMake 전환 불필요
- `libvoikko.a` 정적 라이브러리 -> emcc로 WASM 링크
- `-s MODULARIZE=1 -s EXPORT_NAME="'Libvoikko'"` 설정

## 사전 로딩 전략 (3가지)

| 모드 | 방식 | 용도 |
|------|------|------|
| embed | 사전을 JS 파일에 내장 | Node.js, 오프라인 |
| preload | 별도 .data 파일로 fetch | 브라우저, 효율적 |
| plain | 사전 미포함, 수동 VFS | 커스텀 로딩 |

사전 크기:
- `mor.vfst`: 3.9 MB (형태소 트랜스듀서)
- `autocorr.vfst`: 11 KB
- gzip 시 ~1.5-2 MB

## JS 바인딩 (cwrap 기반)

35개 C API 함수를 `cwrap()`으로 래핑:
- UTF-8 Cstr 변형만 사용 (wchar_t가 JS 경계를 넘지 않음)
- 상위 JS API가 cwrap 레이어를 감싸 idiomatic 객체 제공
- embind 대비 ~15 KB 오버헤드 절감

## 번들 사이즈 예상

| 컴포넌트 | Raw | Gzip |
|----------|----:|-----:|
| WASM 바이너리 | ~500 KB | ~150 KB |
| JS 글루 | ~50 KB | ~15 KB |
| 사전 (mor.vfst) | 3.9 MB | ~1.5 MB |
| 사전 (autocorr.vfst) | 11 KB | ~5 KB |
| **합계** | **~4.5 MB** | **~1.7 MB** |

## 성능

- WASM: native 대비 1.4-2x 오버헤드 (compute-bound)
- FST 트래버설: 순차 메모리 접근, 정수 연산 -> WASM에 적합
- 병목: 문자열 변환 (UTF-8 -> WASM heap 복사), 단어당 ~마이크로초 수준

## 유사 프로젝트 선례

| 프로젝트 | 상태 |
|---------|------|
| libvoikko 자체 | 공식 지원, 위키에 문서화 |
| hunspell-asm | 프로덕션, ~800 KB, Node+Browser |
| ICU4C WASM | 다수 프로젝트에서 사용 |
| Tesseract.js | 프로덕션, OCR via Emscripten |

## 유지보수성

- C++ 소스 변경 0줄 -> upstream 변경 자동 호환
- `js/` 디렉토리만 Emscripten 전용
- 사전 포맷(VFST) 바이너리 안정

## Node.js + Browser 지원

- `MODULARIZE=1`로 두 환경 모두 지원
- Browser: preload 모드, `<script>` 태그
- Node.js: embed 모드, `require()`

## 모던화 작업 목록

| 작업 | 예상 기간 |
|------|----------|
| 최신 Emscripten 빌드 검증 | 1-2시간 |
| ES modules 전환 (`EXPORT_ES6=1`) | 1-2일 |
| TypeScript 선언 파일 (.d.ts) | 1일 |
| npm 패키지 설정 (package.json) | 1일 |
| Fetch 기반 사전 lazy loading | 1-2일 |
| CI/CD (GitHub Actions) | 1일 |
| 테스트 현대화 (QUnit -> vitest) | 1일 |
| 문서화 | 0.5일 |
| **합계** | **~1-2주** |

## 장점

1. 이미 동작하는 빌드 (이론이 아닌 실증)
2. C++ 소스 수정 0줄
3. upstream 자동 추적
4. 전체 API 커버리지 (spell, suggest, hyphenate, grammar, analyze, tokenize)
5. 최소 유지보수 부담

## 단점

1. 사전 크기 (~4 MB) 웹 전송 부담
2. 오래된 빌드 패턴 (ES modules 미지원, npm 패키지 없음)
3. 사전 스트리밍/lazy loading 미구현
4. Tree-shaking 불가 (전체 라이브러리 로드)

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 프로젝트 개요

Corevoikko는 핀란드어 자연어 처리 오픈소스 라이브러리로, 맞춤법 검사, 하이픈 처리, 문법 검사, 형태소 분석을 제공한다. 두 개의 주요 컴포넌트로 구성:
- **libvoikko** — C++ 핵심 라이브러리 (Python, Java 바인딩 포함)
- **voikko-fi** — VFST 형식의 핀란드어 형태소 사전 데이터

라이선스: MPL 1.1 / GPL 2+ / LGPL 2.1+ (tri-license)

## 빌드 명령어

### libvoikko (C++ 라이브러리)

```bash
# Git 체크아웃에서 빌드 시 configure 생성 필요
cd libvoikko
./autogen.sh

# 빌드 (HFST 없이 — macOS에서 hfstospell 미설치 시)
./configure --disable-hfst
make

# HFST 포함 빌드 (hfstospell >= 0.5 필요)
./configure
make
```

C++17 필수. 주요 configure 옵션:
- `--disable-hfst` — HFST 백엔드 비활성화
- `--disable-vfst` — VFST 백엔드 비활성화
- `--enable-expvfst` — 실험적 VFST 기능
- `--enable-vislcg3` — CG3 문법 백엔드 (실험적, tinyxml2 필요)
- `--with-dictionary-path=PATH` — 사전 검색 경로 설정

### voikko-fi (핀란드어 사전)

```bash
cd voikko-fi

# 사전 빌드 (foma, libvoikko, python3, GNU make 필요)
make vvfst

# 설치
make vvfst-install DESTDIR=/usr/lib/voikko
# 또는 사용자 로컬: DESTDIR=~/.voikko

# Sukija 인덱서용 빌드
make vvfst-sukija
```

사전 빌드 튜닝 변수: `VOIKKO_VARIANT`, `GENLEX_OPTS`, `VVFST_BASEFORMS`, `VANHAT_MUODOT`

## 테스트

```bash
cd libvoikko/test

# 외부 사전 없이 실행 가능한 자동 테스트
python3 AllAutomaticTests.py

# 전체 API 테스트 (핀란드어 사전 설치 필요)
python3 libvoikkoTest.py
```

테스트는 Python unittest 기반. `AllAutomaticTests.py`는 `NullComponentTest`와 `DictionaryInfoTest`만 포함하여 외부 의존성 없이 실행된다. `libvoikkoTest.py`는 핀란드어 사전이 설치된 환경에서 전체 공개 API를 테스트한다.

## 아키텍처

### libvoikko 핵심 구조 (`libvoikko/src/`)

공개 API는 `voikko.h`에 정의된 C 함수들이며, Python 바인딩(`libvoikko/python/libvoikko.py`)은 ctypes로 이를 래핑한다.

내부 모듈 구조:
- **fst/** — FST 백엔드 (VFST: 자체 트랜스듀서, HFST: ZHFST 스펠러 아카이브)
- **morphology/** — 형태소 분석 엔진
- **spellchecker/** — 맞춤법 검사, `suggestion/` 하위에 OCR·타이핑 오류 등 제안 전략
- **grammar/** — 문법 검사, `FinnishRuleEngine/`에 핀란드어 규칙
- **hyphenator/** — 음절 기반 하이픈 처리
- **tokenizer/** — 단어/문장 토크나이저
- **setup/** — 사전 로딩 및 초기화

### voikko-fi 사전 구조 (`voikko-fi/vvfst/`)

LEXC 소스 파일들(`*.lexc`)과 foma 스크립트(`*.foma.in`)로 구성. `generate_lex.py`가 XML 어휘(`vocabulary/`)에서 LEXC 렉시콘을 생성하고, foma가 이를 컴파일하여 VFST 바이너리를 만든다.

### 사전 검색 순서

라이브러리는 다음 순서로 사전을 검색한다:
1. `voikkoInit`에 전달된 경로
2. `VOIKKO_DICTIONARY_PATH` 환경변수
3. `~/.voikko` (macOS: `~/Library/Spelling/voikko`도 탐색)
4. `/etc/voikko`
5. `--with-dictionary-path`로 지정한 컴파일 타임 경로

## 컴파일러 플래그

`configure.ac`에서 `-Wall -Werror -pedantic`이 기본 설정되어 있어 경고가 에러로 처리된다.

## 도구 (`tools/bin/`)

- `voikkotest` — 종합 테스트 스위트
- `voikko-build-dicts` — 커스텀 설정으로 사전 빌드
- `voikko-inflect-word` — 형태소 굴절 도구
- `voikko-gc-pretty` — 문법 검사 출력 포매터

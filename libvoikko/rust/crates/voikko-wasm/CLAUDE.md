# voikko-wasm

wasm-bindgen 기반 WASM 바인딩. 가장 얇은 레이어.

## 역할 (Phase 5)

- `WasmVoikko` struct → `#[wasm_bindgen]` export
- 15개 메서드 + 14개 옵션 setter
- `Result<T, JsError>` 에러 변환
- 복합 타입은 `serde-wasm-bindgen`으로 JS 변환
- FinalizationRegistry로 자동 cleanup

## 빌드

```bash
cargo build --target wasm32-unknown-unknown --release -p voikko-wasm
wasm-bindgen target/wasm32-unknown-unknown/release/voikko_wasm.wasm \
  --out-dir pkg --target bundler --typescript
wasm-opt pkg/voikko_wasm_bg.wasm -Oz -o pkg/voikko_wasm_bg.wasm
```

## crate-type

`cdylib` — WASM 바이너리 생성용

상세 패턴: `plan/phase2-rust/04-rust-ecosystem.md`

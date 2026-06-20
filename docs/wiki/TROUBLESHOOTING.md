# Troubleshooting

## BAT에서 빌드하지 못함

BAT 파일은 앱 실행 전에 자동으로 `cargo build`를 수행한다. Cargo를 찾지 못한다는 오류가 나오면 [rustup](https://rustup.rs/)으로 Rust를 설치하고 새 콘솔에서 다시 실행한다.

## 설정 파일 오류

`--config`로 지정한 파일은 반드시 존재하고 유효한 TOML이어야 한다. 설정 파일을 생략하면 내장 기본값 `mock + null`을 사용한다.

## DSX 반응 없음

- DSX의 UDP 서버가 활성화되었는지 확인한다.
- `C:\Temp\DualSenseX\DualSenseX_PortNumber.txt`의 포트를 확인하거나 `--dsx-port`로 직접 지정한다.
- 기본 controller index는 0이다.

## LMU가 항상 disconnected

v0.1의 정상 동작이다. 실제 `LMU_Data` 연결과 구조 해석은 v0.2 TODO다.

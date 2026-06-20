# QA Guide

## v0.1 자동/로컬 QA

1. `cargo build`, `cargo clippy --all-targets -- -D warnings`, `cargo test`가 성공하는지 확인한다.
2. `run_mock.bat`에서 telemetry와 null-output 로그가 약 1초 간격으로 출력되는지 확인한다.
3. `--telemetry mock --output dsx-udp` 실행 시 DSX가 꺼져 있어도 앱이 종료되지 않는지 확인한다.
4. DSX UDP를 활성화한 뒤 mock 시나리오가 L2/R2 효과로 반복되는지 확인한다.

## v0.2 실제 플레이 QA

LMU 연결, 일반 제동, ABS, 일반 가속, TC, rev limiter를 각각 별도 상황에서 확인한다. 실제 shared-memory reader가 구현되기 전에는 이 항목을 완료 처리하지 않는다.


# QA Guide

## 자동 검증

```powershell
cargo fmt -- --check
cargo build
cargo clippy --all-targets -- -D warnings
cargo test
```

## 실제 플레이 QA

실행 전 Steam DSX의 UDP 서버를 활성화하고 DualSense가 controller index 0으로 연결되어 있는지 확인한다.

현재 상태: 자동 검증 및 실제 LMU+DSX+DualSense QA 완료.

결과: 연결, 자동 재연결, 일반 브레이크/스로틀 저항, ABS/TC pulse, rev-limit vibration이 정상 작동했다. 답력과 pulse 강도의 사용자 조절 기능은 향후 GUI 단계에서 추가한다.

### QA-01 연결과 재연결

1. `run_lmu.bat`을 먼저 실행한다.
2. LMU를 실행하고 차량에 탑승한다.
3. `connected to LMU shared memory`, `LMU telemetry is active`, `player vehicle telemetry acquired` 로그를 확인한다.
4. LMU를 종료한다.
5. 앱이 종료되지 않고 양쪽 트리거가 Normal로 돌아오는지 확인한다.
6. LMU를 다시 실행해 2초 이내에 재연결되는지 확인한다.

### QA-02 일반 제동

ABS가 개입하지 않는 직선 제동에서 brake가 깊어질수록 L2 저항이 증가해야 한다.

### QA-03 ABS

ABS 차량으로 강한 제동을 유도한다. ABS 개입 중 L2에서 일반 저항과 구분되는 pulse가 느껴져야 한다.

### QA-04 일반 가속

정상 가속에서 throttle 입력량에 따라 R2에 약한 저항이 생겨야 한다.

### QA-05 TC

저속 코너 탈출에서 TC 개입을 유도한다. TC active 동안 R2 pulse가 느껴져야 한다.

### QA-06 Rev limiter

RPM을 rev limit 부근에 유지한다. TC보다 우선하는 R2 vibration이 느껴져야 한다.

### QA 결과 기록

아래 항목을 기록한다.

- 연결/재연결 성공 여부
- L2 일반 저항 강도
- ABS pulse 구분 가능 여부
- R2 일반 저항 강도
- TC pulse 구분 가능 여부
- rev-limit vibration 구분 가능 여부
- 너무 강하거나 약한 효과와 사용 차량

강도 기본값은 현재 상태로 유지하며, 사용자 조절 기능은 향후 GUI에서 제공한다.

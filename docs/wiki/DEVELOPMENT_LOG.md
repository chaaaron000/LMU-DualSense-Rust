# Development Log

## v0.2

- 실제 `LMU_Data` Win32 mapping과 공식 shared lock 추가
- LMU 프로세스 종료 감지 및 2초 자동 재연결 추가
- pyLMUSharedMemory 기준 checked byte parser 추가
- player vehicle의 unfiltered pedal, RPM, gear, ABS/TC, speed limiter 추출
- 연결 해제 시 smoothing 및 adaptive trigger 상태 초기화
- DSX Pulse/Vibrate를 Machine mode 18로 변경해 강도와 frequency 전달
- synthetic snapshot, layout validation, reconnect-safe effect 테스트 추가
- 실제 LMU+DSX+DualSense QA에서 모든 효과와 자동 재연결 동작 확인
- 답력 및 pulse 강도 조절 기능은 향후 GUI 범위로 이관
- ABS가 없는 차량을 위해 TinyPedal Trailing 방식의 동적 타이어 반경 보정과 wheel slip 기반 L2 pulse 추가
- raw brake 2%, wheel lock 30%, 최소 차속 1 m/s 기준을 적용하고 `mGripFract` 보조 판정 제거
- TC가 없는 차량을 위해 raw throttle 2%, wheel slip 10% 기준의 R2 pulse 추가

## v0.1

- Rust CLI, TOML 설정, mock telemetry와 표준 `TelemetryFrame` 추가
- brake/ABS 및 throttle/TC/rev-limit effect mapping 추가
- null output과 Steam DSX UDP v2 output 추가
- BAT 자동 빌드와 UDP loopback 테스트 추가

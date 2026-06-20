# Architecture

```text
LMU process
  → LMU_Data + official shared lock
  → 324,820-byte snapshot
  → checked byte parser
  → TelemetryFrame
  → EffectMapper
  → TriggerOutputFrame
  → Steam DSX UDP v2
  → DualSense
```

## LMU reader

- Windows process handle로 LMU 종료를 감지한다.
- `LMU_SharedMemoryLockData`와 `LMU_SharedMemoryLockEvent`로 snapshot 복사를 동기화한다.
- `unsafe`는 Win32 handle, mapped view, byte 복사 계층에만 존재한다.
- parser는 4-byte packed layout의 크기, 차량 수, player index, bool, 숫자 범위를 검증한다.
- LMU가 없거나 종료되면 양쪽 트리거를 Normal로 만들고 2초마다 재연결한다.

## Effects

- L2: ABS pulse → brake resistance → Normal
- R2: rev-limit vibration → TC pulse → throttle resistance → Normal
- smoothing은 연결 해제 또는 차량 하차 시 초기화한다.

내부 효과 값은 `0..=10`이며 DSX encoder만 protocol 범위로 변환한다.

